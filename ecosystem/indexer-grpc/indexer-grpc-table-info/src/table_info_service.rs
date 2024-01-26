// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::backup_restore::gcs::GcsBackupRestoreOperator;
use anyhow::Error;
use aptos_api::context::Context;
use aptos_api_types::TransactionOnChainData;
use aptos_db_indexer::db_v2::IndexerAsyncV2;
use aptos_indexer_grpc_fullnode::stream_coordinator::{
    IndexerStreamCoordinator, TransactionBatchInfo,
};
use aptos_indexer_grpc_utils::counters::{log_grpc_step, IndexerGrpcStep};
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_types::write_set::WriteSet;
use std::{sync::Arc, time::Duration};
use tonic::Status;

type EndVersion = u64;
const LEDGER_VERSION_RETRY_TIME_MILLIS: u64 = 10;
const SERVICE_TYPE: &str = "table_info_service";

pub struct TableInfoService {
    pub current_version: u64,
    pub parser_task_count: u16,
    pub parser_batch_size: u16,
    pub context: Arc<Context>,
    pub enable_expensive_logging: bool,
    pub backup_restore_operator: Option<Arc<GcsBackupRestoreOperator>>,
    pub indexer_async_v2: Arc<IndexerAsyncV2>,
}

impl TableInfoService {
    pub fn new(
        context: Arc<Context>,
        request_start_version: u64,
        parser_task_count: u16,
        parser_batch_size: u16,
        enable_expensive_logging: bool,
        backup_restore_operator: Option<Arc<GcsBackupRestoreOperator>>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
    ) -> Self {
        Self {
            current_version: request_start_version,
            parser_task_count,
            parser_batch_size,
            context,
            enable_expensive_logging,
            backup_restore_operator,
            indexer_async_v2,
        }
    }

    /// 1. fetch new transactions
    /// 2. break them down into batches in parser_batch_size and spawn up threads in parser_task_count
    /// 3. parse write sets from transactions with move annotater to get table handle -> key, value type
    /// 4. write parsed table info to rocksdb
    /// 5. after all batches from the loop complete, if pending on items not empty, move on to 6, otherwise, start from 1 again
    /// 6. retry all the txns in the loop sequentially to clean up the pending on items
    /// 7. try to backup rocksdb snapshot if new epoch have been found
    pub async fn run(&mut self) {
        loop {
            let start_time = std::time::Instant::now();
            let ledger_version = self.get_highest_known_version().await.unwrap_or_default();
            let batches = self.get_batches(ledger_version).await;
            let results = self
                .process_multiple_batches(self.indexer_async_v2.clone(), batches, ledger_version)
                .await;
            let max_version = self.get_max_batch_version(results).unwrap_or_default();
            let versions_processed = max_version - self.current_version + 1;
            let context = self.context.clone();
            let backup_restore_operator = self.backup_restore_operator.clone();
            let start_version = self.current_version;
            let indexer_async_v2 = self.indexer_async_v2.clone();

            // Try uploading the rocksdb snapshot by taking a full db checkpoint and save it to gcs if found new epoch
            // running backup logic in a separate thread to not let it block the main thread to parse table info, since
            // gcs operation could be slow
            if let Some(op) = backup_restore_operator {
                tokio::spawn(async move {
                    Self::try_backup_db_snapshot(
                        context.clone(),
                        max_version,
                        indexer_async_v2.clone(),
                        op.clone(),
                    )
                    .await;
                });
            }

            log_grpc_step(
                SERVICE_TYPE,
                IndexerGrpcStep::TableInfoProcessed,
                Some(start_version as i64),
                Some(max_version as i64),
                None,
                None,
                Some(start_time.elapsed().as_secs_f64()),
                None,
                Some(versions_processed as i64),
                None,
            );

            self.current_version = max_version + 1;
        }
    }

    /// Fans out a bunch of threads and processes write sets from transactions in parallel.
    /// Pushes results in parallel to the stream, but only return that the batch is
    /// fully completed if every job in the batch is successful and no pending on items
    /// Processing transactions in 2 stages:
    /// 1. Fetch transactions from ledger db
    /// 2. Get write sets from transactions and parse write sets to get handle -> key,value type mapping, write the mapping to the rocksdb
    async fn process_multiple_batches(
        &self,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        batches: Vec<TransactionBatchInfo>,
        ledger_version: u64,
    ) -> Vec<Result<EndVersion, Status>> {
        let mut tasks = vec![];
        let context = self.context.clone();

        for batch in batches.iter().cloned() {
            let task = tokio::spawn(Self::process_single_batch(
                context.clone(),
                indexer_async_v2.clone(),
                ledger_version,
                batch,
                false, /* end_early_if_pending_on_empty */
                self.enable_expensive_logging,
            ));
            tasks.push(task);
        }

        match futures::future::try_join_all(tasks).await {
            Ok(res) => {
                let last_batch = batches.last().cloned().unwrap();
                let total_txns_to_process = last_batch.start_version
                    + last_batch.num_transactions_to_fetch as u64
                    - self.current_version;
                let end_version =
                    last_batch.start_version + last_batch.num_transactions_to_fetch as u64;

                // Clean up pending on items across threads
                self.indexer_async_v2
                    .cleanup_pending_on_items()
                    .expect("[Table Info] Failed to clean up the pending on items");

                // If pending on items are not empty, meaning the current loop hasn't fully parsed all table infos
                // due to the nature of multithreading where instructions used to parse table info might come later,
                // retry sequentially to ensure parsing is complete
                //
                // Risk of this sequential approach is that it could be slow when the txns to process contain extremely
                // nested table items, but the risk is bounded by the configuration of the number of txns to process and number of threads
                if !self.indexer_async_v2.is_indexer_async_v2_pending_on_empty() {
                    let retry_batch = TransactionBatchInfo {
                        start_version: self.current_version,
                        num_transactions_to_fetch: total_txns_to_process as u16,
                        head_version: ledger_version,
                    };

                    Self::process_single_batch(
                        context.clone(),
                        indexer_async_v2.clone(),
                        ledger_version,
                        retry_batch,
                        true, /* end_early_if_pending_on_empty */
                        self.enable_expensive_logging,
                    )
                    .await
                    .expect("[Table Info] Failed to parse table info");
                }

                assert!(
                    self.indexer_async_v2.is_indexer_async_v2_pending_on_empty(),
                    "Missing data in table info parsing after sequential retry"
                );

                // Update rocksdb's to be processed next version after verifying all txns are successfully parsed
                self.indexer_async_v2
                    .update_next_version(end_version + 1)
                    .unwrap();

                res
            },
            Err(err) => panic!(
                "[Table Info] Error processing table info batches: {:?}",
                err
            ),
        }
    }

    /// Process a single batch of transactions for table info parsing.
    /// It's used in the first loop to process batches in parallel,
    /// and it's used in the second loop to process transactions sequentially
    /// if pending on items are not empty
    async fn process_single_batch(
        context: Arc<Context>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        ledger_version: u64,
        batch: TransactionBatchInfo,
        end_early_if_pending_on_empty: bool,
        _enable_verbose_logging: bool,
    ) -> Result<EndVersion, Status> {
        let start_time = std::time::Instant::now();

        let raw_txns = IndexerStreamCoordinator::fetch_raw_txns_with_retries(
            context.clone(),
            ledger_version,
            batch,
        )
        .await;

        Self::parse_table_info(
            context.clone(),
            raw_txns.clone(),
            indexer_async_v2,
            end_early_if_pending_on_empty,
        )
        .expect("[Table Info] Failed to parse table info");

        log_grpc_step(
            SERVICE_TYPE,
            IndexerGrpcStep::TableInfoProcessedBatch,
            Some(batch.start_version as i64),
            Some((batch.start_version + batch.num_transactions_to_fetch as u64) as i64),
            None,
            None,
            Some(start_time.elapsed().as_secs_f64()),
            None,
            Some(batch.num_transactions_to_fetch as i64),
            None,
        );

        Ok(raw_txns.last().unwrap().version)
    }

    /// Retrieves transaction batches based on the provided ledger version.
    /// The function prepares to fetch transactions by determining the start version,
    /// the number of fetches, and the size of each batch.
    async fn get_batches(&mut self, ledger_version: u64) -> Vec<TransactionBatchInfo> {
        info!(
            current_version = self.current_version,
            highest_known_version = ledger_version,
            parser_batch_size = self.parser_batch_size,
            parser_task_count = self.parser_task_count,
            "[Table Info] Preparing to fetch transactions"
        );

        let mut start_version = self.current_version;
        let mut num_fetches = 0;
        let mut batches = vec![];

        while num_fetches < self.parser_task_count && start_version <= ledger_version {
            let num_transactions_to_fetch = std::cmp::min(
                self.parser_batch_size as u64,
                ledger_version - start_version + 1,
            ) as u16;

            batches.push(TransactionBatchInfo {
                start_version,
                num_transactions_to_fetch,
                head_version: ledger_version,
            });

            start_version += num_transactions_to_fetch as u64;
            num_fetches += 1;
        }

        batches
    }

    fn get_max_batch_version(
        &self,
        results: Vec<Result<EndVersion, Status>>,
    ) -> Option<EndVersion> {
        results
            .into_iter()
            .map(|result| result.ok())
            .max()
            .unwrap_or_default()
    }

    /// Parse table info from write sets,
    /// end_early_if_pending_on_empty flag will be true if we couldn't parse all table infos in the first try with multithread,
    /// in the second try with sequential looping, to make parsing efficient, we end early if all table infos are parsed
    fn parse_table_info(
        context: Arc<Context>,
        raw_txns: Vec<TransactionOnChainData>,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        end_early_if_pending_on_empty: bool,
    ) -> Result<(), Error> {
        if raw_txns.is_empty() {
            return Ok(());
        }

        let start_time = std::time::Instant::now();
        let first_version = raw_txns.first().map(|txn| txn.version).unwrap();
        let write_sets: Vec<WriteSet> = raw_txns.iter().map(|txn| txn.changes.clone()).collect();
        let write_sets_slice: Vec<&WriteSet> = write_sets.iter().collect();
        indexer_async_v2
            .index_table_info(
                context.db.clone(),
                first_version,
                &write_sets_slice,
                end_early_if_pending_on_empty,
            )
            .expect(
                "[Table Info] Failed to process write sets and index to the table info rocksdb",
            );

        info!(
            table_info_first_version = first_version,
            table_info_parsing_millis_per_batch = start_time.elapsed().as_millis(),
            num_transactions = raw_txns.len(),
            "[Table Info] Table info parsed successfully"
        );

        Ok(())
    }

    /// Tries to upload a snapshot of the database if the backup service is enabled.
    /// This function is called to periodically back up the database state to Google Cloud Storage (GCS).
    /// It checks the latest epoch of data already backed up in GCS and compares it with the current epoch.
    async fn try_backup_db_snapshot(
        context: Arc<Context>,
        last_version: u64,
        indexer_async_v2: Arc<IndexerAsyncV2>,
        backup_restore_operator: Arc<GcsBackupRestoreOperator>,
    ) {
        // only try backup db if backup service is enabled
        let metadata_epoch = backup_restore_operator
            .clone()
            .get_metadata_epoch();
        let (_, _, block_event) = context
            .db
            .get_block_info_by_version(last_version)
            .unwrap_or_else(|_| {
                panic!("Could not get block_info for last version {}", last_version,)
            });
        let block_event_epoch = block_event.epoch();
        // If gcs most recent transaction version in metadata is behind, take a snapshot of rocksdb and upload
        if metadata_epoch < block_event_epoch {
            let start_time = std::time::Instant::now();
            // temporary path to store the snapshot
            let snapshot_dir = context
                .node_config
                .get_data_dir()
                .join(block_event_epoch.to_string());
            let ledger_chain_id = context.chain_id().id();
            backup_restore_operator
                .backup_db_snapshot(
                    ledger_chain_id as u64,
                    block_event_epoch,
                    indexer_async_v2,
                    snapshot_dir.clone(),
                )
                .await
                .expect("Failed to upload snapshot in table info service");

            info!(
                backup_epoch = block_event_epoch,
                backup_millis = start_time.elapsed().as_millis(),
                "[Table Info] Table info db backed up successfully"
            );
        }
    }

    /// TODO(jill): consolidate it with `ensure_highest_known_version`
    /// Will keep looping and checking the latest ledger info to see if there are new transactions
    /// If there are, it will update the ledger version version
    async fn get_highest_known_version(&self) -> Result<u64, Error> {
        let mut info = self.context.get_latest_ledger_info_wrapped();
        let mut ledger_version = info.unwrap().ledger_version.0;
        let mut empty_loops = 0;

        while ledger_version == 0 || self.current_version > ledger_version {
            if empty_loops > 0 {
                tokio::time::sleep(Duration::from_millis(LEDGER_VERSION_RETRY_TIME_MILLIS)).await;
            }
            empty_loops += 1;
            if let Err(err) = {
                info = self.context.get_latest_ledger_info_wrapped();
                ledger_version = info.unwrap().ledger_version.0;
                Ok::<(), Error>(())
            } {
                error!(
                    error = format!("{:?}", err),
                    "[Table Info] Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(100),
                    debug!(
                        ledger_version = ledger_version,
                        "[Table Info] Found new highest known ledger version",
                    )
                );
            }
        }
        Ok(ledger_version)
    }
}
