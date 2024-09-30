// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    block_preparer::BlockPreparer,
    counters::{self, log_executor_error_occurred},
    monitor,
    pipeline::pipeline_phase::CountedRequest,
    state_computer::StateComputeResultFut,
};
use aptos_consensus_types::{
    block::Block,
    pipeline_execution_result::PipelineExecutionResult,
    pipelined_block::{OrderedBlockWindow, PipelinedBlock},
};
use aptos_crypto::HashValue;
use aptos_executor_types::{
    state_checkpoint_output::StateCheckpointOutput, BlockExecutorTrait, ExecutorError,
    ExecutorResult,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::{debug, info, warn};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        partitioner::{ExecutableBlock, ExecutableTransactions},
    },
    block_metadata_ext::BlockMetadataExt,
    transaction::{
        signature_verified_transaction::{
            SignatureVerifiedTransaction,
            SignatureVerifiedTransaction::{Invalid, Valid},
        },
        SignedTransaction,
        Transaction::UserTransaction,
        TransactionStatus,
    },
    txn_provider::{
        blocking_txns_provider::{BlockingTransaction, BlockingTxnsProvider},
        TxnIndex,
    },
};
use fail::fail_point;
use futures::future::BoxFuture;
use itertools::Itertools;
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{
    collections::HashSet,
    iter::zip,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

#[allow(clippy::unwrap_used)]
pub static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(8) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("signature-checker-{}", index))
            .build()
            .unwrap(),
    )
});

pub struct ExecutionPipeline {
    prepare_block_tx: mpsc::UnboundedSender<PrepareBlockCommand>,
}

impl ExecutionPipeline {
    pub fn spawn(
        executor: Arc<dyn BlockExecutorTrait>,
        runtime: &tokio::runtime::Handle,
        enable_pre_commit: bool,
    ) -> Self {
        let (prepare_block_tx, prepare_block_rx) = mpsc::unbounded_channel();
        let (execute_block_tx, execute_block_rx) = mpsc::unbounded_channel();
        let (ledger_apply_tx, ledger_apply_rx) = mpsc::unbounded_channel();
        let (pre_commit_tx, pre_commit_rx) = mpsc::unbounded_channel();

        runtime.spawn(Self::prepare_block_stage(
            prepare_block_rx,
            execute_block_tx,
        ));
        runtime.spawn(Self::execute_stage(
            execute_block_rx,
            ledger_apply_tx,
            executor.clone(),
        ));
        runtime.spawn(Self::ledger_apply_stage(
            ledger_apply_rx,
            pre_commit_tx,
            executor.clone(),
            enable_pre_commit,
        ));
        runtime.spawn(Self::pre_commit_stage(pre_commit_rx, executor));

        Self { prepare_block_tx }
    }

    pub async fn queue(
        &self,
        block: PipelinedBlock,
        block_window: OrderedBlockWindow,
        metadata: BlockMetadataExt,
        parent_block_id: HashValue,
        txn_generator: BlockPreparer,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        lifetime_guard: CountedRequest<()>,
    ) -> StateComputeResultFut {
        let (result_tx, result_rx) = oneshot::channel();
        let block_round = block.round();
        let block_id = block.id();
        self.prepare_block_tx
            .send(PrepareBlockCommand {
                block,
                block_window,
                metadata,
                block_executor_onchain_config,
                parent_block_id,
                block_preparer: txn_generator,
                result_tx,
                command_creation_time: Instant::now(),
                lifetime_guard,
            })
            .expect("Failed to send block to execution pipeline.");

        Box::pin(async move {
            let result = result_rx
                .await
                .map_err(|err| ExecutorError::InternalError {
                    error: format!(
                        "Failed to receive execution result for block {}: {:?}.",
                        block_id, err
                    ),
                })?;
            info!(
                "received result_rx for round {} block {}.",
                block_round, block_id
            );
            result
        })
    }

    async fn prepare_block(
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
        command: PrepareBlockCommand,
    ) {
        let PrepareBlockCommand {
            block,
            block_window,
            metadata,
            block_executor_onchain_config,
            parent_block_id,
            block_preparer,
            result_tx,
            command_creation_time,
            lifetime_guard,
        } = command;
        counters::PREPARE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
        debug!("prepare_block received block {}.", block.id());
        let input_txns = block_preparer
            .prepare_block(block.block(), &block_window)
            .await;
        if let Err(e) = input_txns {
            result_tx
                .send(Err(e))
                .unwrap_or_else(log_failed_to_send_result("prepare_block", block.id()));
            return;
        }
        let validator_txns = block.validator_txns().cloned().unwrap_or_default();
        let input_txns = input_txns.expect("input_txns must be Some.");
        tokio::task::spawn_blocking(move || {
            let txns_to_execute =
                Block::combine_to_input_transactions(validator_txns, input_txns.clone(), metadata);
            let sig_verification_start = Instant::now();
            let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
                SIG_VERIFY_POOL.install(|| {
                    let num_txns = txns_to_execute.len();
                    txns_to_execute
                        .into_par_iter()
                        .with_min_len(optimal_min_len(num_txns, 32))
                        .map(|t| t.into())
                        .collect::<Vec<_>>()
                });
            counters::PREPARE_BLOCK_SIG_VERIFICATION_TIME
                .observe_duration(sig_verification_start.elapsed());
            let block_id = block.id();
            execute_block_tx
                .send(ExecuteBlockCommand {
                    input_txns,
                    pipelined_block: block,
                    block: (block_id, sig_verified_txns).into(),
                    block_window,
                    parent_block_id,
                    block_executor_onchain_config,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                })
                .expect("Failed to send block to execution pipeline.");
        })
        .await
        .expect("Failed to spawn_blocking.");
    }

    async fn prepare_block_stage(
        mut prepare_block_rx: mpsc::UnboundedReceiver<PrepareBlockCommand>,
        execute_block_tx: mpsc::UnboundedSender<ExecuteBlockCommand>,
    ) {
        while let Some(command) = prepare_block_rx.recv().await {
            monitor!(
                "prepare_block",
                Self::prepare_block(execute_block_tx.clone(), command).await
            );
        }
        debug!("prepare_block_stage quitting.");
    }

    async fn execute_stage(
        mut block_rx: mpsc::UnboundedReceiver<ExecuteBlockCommand>,
        ledger_apply_tx: mpsc::UnboundedSender<LedgerApplyCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        while let Some(ExecuteBlockCommand {
            input_txns,
            pipelined_block,
            block,
            block_window,
            parent_block_id,
            block_executor_onchain_config,
            result_tx,
            command_creation_time,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            counters::EXECUTE_BLOCK_WAIT_TIME.observe_duration(command_creation_time.elapsed());
            let block_id = block.block_id;
            info!("execute_stage received block {}.", block_id);

            let now = Instant::now();
            let mut committed_transactions = HashSet::new();

            // TODO: lots of repeated code here
            monitor!("execute_wait_for_committed_transactions", {
                block_window
                    .pipelined_blocks()
                    .iter()
                    .filter(|window_block| window_block.round() == pipelined_block.round() - 1)
                    .for_each(|b| {
                        info!(
                        "Execution: Waiting for committed transactions at block {} for block {}",
                        b.round(),
                        pipelined_block.round()
                    );
                        for txn_hash in b.wait_for_committed_transactions() {
                            committed_transactions.insert(*txn_hash);
                        }
                    });
            });

            let (input_txns, block) = monitor!("execute_filter_committed_transactions", {
                let prev_input_txns_len = input_txns.len();
                let input_txns: Vec<_> = input_txns
                    .into_iter()
                    .filter(|txn| !committed_transactions.contains(&txn.committed_hash()))
                    .collect();
                info!(
                    "Execution: Filtered out {}/{} transactions from the previous block for block {}, in {} ms",
                    prev_input_txns_len - input_txns.len(),
                    prev_input_txns_len,
                    pipelined_block.round(),
                    now.elapsed().as_millis()
                );

                // TODO: Find a better way to do this.
                let (mut txns, blocking_txns_provider) = match block.transactions {
                    ExecutableTransactions::Unsharded(txns) => {
                        let transactions: Vec<_> = txns
                            .into_iter()
                            .filter(|txn| {
                                if let Valid(UserTransaction(user_txn)) = txn {
                                    !committed_transactions.contains(&user_txn.committed_hash())
                                } else {
                                    true
                                }
                            })
                            .collect();
                        let blocking_txns: Vec<_> = (0..transactions.len())
                            .map(|_| BlockingTransaction::new())
                            .collect();
                        (
                            transactions,
                            Arc::new(BlockingTxnsProvider::new(blocking_txns)),
                        )
                    },
                    ExecutableTransactions::UnshardedBlocking(_) => {
                        unimplemented!("Not expecting this yet.")
                    },
                    ExecutableTransactions::Sharded(_) => {
                        unimplemented!("Sharded transactions are not supported yet.")
                    },
                };
                let blocking_txns_writer = blocking_txns_provider.clone();
                tokio::spawn(async move {
                    tokio::task::spawn_blocking(move || {
                        // TODO: keep this previously split so we don't have to re-split it here
                        if let Some((first_user_txn_idx, _)) = txns.iter().find_position(|txn| {
                            let txn = match txn {
                                Valid(txn) => txn,
                                Invalid(txn) => txn,
                            };
                            matches!(txn, UserTransaction(_))
                        }) {
                            let validator_txns: Vec<_> = txns.drain(0..first_user_txn_idx).collect();
                            let shuffle_iterator = crate::transaction_shuffler::use_case_aware::iterator::ShuffledTransactionIterator::new(crate::transaction_shuffler::use_case_aware::Config {
                                sender_spread_factor: 32,
                                platform_use_case_spread_factor: 0,
                                user_use_case_spread_factor: 4,
                            }).extended_with(txns);
                            for (idx, txn) in validator_txns
                                .into_iter()
                                .chain(shuffle_iterator)
                                .enumerate()
                            {
                                blocking_txns_writer.set_txn(idx as TxnIndex, txn);
                            }
                        } else {
                            // No user transactions in the block.
                            for (idx, txn) in txns.into_iter().enumerate() {
                                blocking_txns_writer.set_txn(idx as TxnIndex, txn);
                            }
                        }
                    }).await.expect("Failed to spawn_blocking.");
                });
                let transactions =
                    ExecutableTransactions::UnshardedBlocking(blocking_txns_provider);
                let block = ExecutableBlock::new(block.block_id, transactions);
                (input_txns, block)
            });

            let executor = executor.clone();
            let state_checkpoint_output = monitor!(
                "execute_block",
                tokio::task::spawn_blocking(move || {
                    fail_point!("consensus::compute", |_| {
                        Err(ExecutorError::InternalError {
                            error: "Injected error in compute".into(),
                        })
                    });
                    let start = Instant::now();
                    info!("execute_and_state_checkpoint start. {}", block_id);
                    executor
                        .execute_and_state_checkpoint(
                            block,
                            parent_block_id,
                            block_executor_onchain_config,
                        )
                        .map(|output| {
                            info!("execute_and_state_checkpoint end. {}", block_id);
                            (output, start.elapsed())
                        })
                })
                .await
            )
            .expect("Failed to spawn_blocking.");

            if let Ok((output, _)) = &state_checkpoint_output {
                // Block metadata + validator transactions
                let num_system_txns = 1 + pipelined_block
                    .validator_txns()
                    .map_or(0, |txns| txns.len());
                let committed_transactions: Vec<_> = zip(
                    input_txns.iter(),
                    output
                        .txns
                        .statuses_for_input_txns
                        .iter()
                        .skip(num_system_txns),
                )
                .filter_map(|(input_txn, txn_status)| {
                    if let TransactionStatus::Keep(_) = txn_status {
                        Some(input_txn.committed_hash())
                    } else {
                        None
                    }
                })
                .collect();
                pipelined_block.set_committed_transactions(committed_transactions);
            } else {
                pipelined_block.cancel_committed_transactions();
            }

            ledger_apply_tx
                .send(LedgerApplyCommand {
                    input_txns,
                    block_id,
                    parent_block_id,
                    state_checkpoint_output,
                    result_tx,
                    command_creation_time: Instant::now(),
                    lifetime_guard,
                })
                .expect("Failed to send block to ledger_apply stage.");
        }
        debug!("execute_stage quitting.");
    }

    async fn ledger_apply_stage(
        mut block_rx: mpsc::UnboundedReceiver<LedgerApplyCommand>,
        pre_commit_tx: mpsc::UnboundedSender<PreCommitCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
        enable_pre_commit: bool,
    ) {
        while let Some(LedgerApplyCommand {
            input_txns,
            block_id,
            parent_block_id,
            state_checkpoint_output: execution_result,
            result_tx,
            command_creation_time,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            counters::APPLY_LEDGER_WAIT_TIME.observe_duration(command_creation_time.elapsed());
            info!("ledger_apply stage received block {}.", block_id);
            let res = async {
                let (state_checkpoint_output, execution_duration) = execution_result?;
                let executor = executor.clone();
                monitor!(
                    "ledger_apply",
                    tokio::task::spawn_blocking(move || {
                        executor.ledger_update(block_id, parent_block_id, state_checkpoint_output)
                    })
                    .await
                )
                .expect("Failed to spawn_blocking().")
                .map(|output| (output, execution_duration))
            }
            .await;
            let pipeline_res = res.map(|(output, execution_duration)| {
                let pre_commit_fut: BoxFuture<'static, ExecutorResult<()>> =
                    if output.epoch_state().is_some() || !enable_pre_commit {
                        // hack: it causes issue if pre-commit is finished at an epoch ending, and
                        // we switch to state sync, so we do the pre-commit only after we actually
                        // decide to commit (in the commit phase)
                        let executor = executor.clone();
                        Box::pin(async move {
                            tokio::task::spawn_blocking(move || {
                                executor.pre_commit_block(block_id, parent_block_id)
                            })
                            .await
                            .expect("failed to spawn_blocking")
                        })
                    } else {
                        // kick off pre-commit right away
                        let (pre_commit_result_tx, pre_commit_result_rx) = oneshot::channel();
                        // schedule pre-commit
                        pre_commit_tx
                            .send(PreCommitCommand {
                                block_id,
                                parent_block_id,
                                result_tx: pre_commit_result_tx,
                                lifetime_guard,
                            })
                            .expect("Failed to send block to pre_commit stage.");
                        Box::pin(async {
                            pre_commit_result_rx
                                .await
                                .map_err(ExecutorError::internal_err)?
                        })
                    };

                PipelineExecutionResult::new(input_txns, output, execution_duration, pre_commit_fut)
            });
            result_tx
                .send(pipeline_res)
                .unwrap_or_else(log_failed_to_send_result("ledger_apply", block_id));
        }
        debug!("ledger_apply stage quitting.");
    }

    async fn pre_commit_stage(
        mut block_rx: mpsc::UnboundedReceiver<PreCommitCommand>,
        executor: Arc<dyn BlockExecutorTrait>,
    ) {
        while let Some(PreCommitCommand {
            block_id,
            parent_block_id,
            result_tx,
            lifetime_guard,
        }) = block_rx.recv().await
        {
            debug!("pre_commit stage received block {}.", block_id);
            let res = async {
                let executor = executor.clone();
                monitor!(
                    "pre_commit",
                    tokio::task::spawn_blocking(move || {
                        executor.pre_commit_block(block_id, parent_block_id)
                    })
                )
                .await
                .expect("Failed to spawn_blocking().")
            }
            .await;
            result_tx
                .send(res)
                .unwrap_or_else(log_failed_to_send_result("pre_commit", block_id));
            drop(lifetime_guard);
        }
        debug!("pre_commit stage quitting.");
    }
}

struct PrepareBlockCommand {
    block: PipelinedBlock,
    block_window: OrderedBlockWindow,
    metadata: BlockMetadataExt,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    // The parent block id.
    parent_block_id: HashValue,
    block_preparer: BlockPreparer,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct ExecuteBlockCommand {
    input_txns: Vec<SignedTransaction>,
    pipelined_block: PipelinedBlock,
    block: ExecutableBlock,
    block_window: OrderedBlockWindow,
    parent_block_id: HashValue,
    block_executor_onchain_config: BlockExecutorConfigFromOnchain,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct LedgerApplyCommand {
    input_txns: Vec<SignedTransaction>,
    block_id: HashValue,
    parent_block_id: HashValue,
    state_checkpoint_output: ExecutorResult<(StateCheckpointOutput, Duration)>,
    result_tx: oneshot::Sender<ExecutorResult<PipelineExecutionResult>>,
    command_creation_time: Instant,
    lifetime_guard: CountedRequest<()>,
}

struct PreCommitCommand {
    block_id: HashValue,
    parent_block_id: HashValue,
    result_tx: oneshot::Sender<ExecutorResult<()>>,
    lifetime_guard: CountedRequest<()>,
}

fn log_failed_to_send_result<T>(
    from_stage: &'static str,
    block_id: HashValue,
) -> impl FnOnce(ExecutorResult<T>) {
    move |value| {
        warn!(
            from_stage = from_stage,
            block_id = block_id,
            is_err = value.is_err(),
            "Failed to send back execution/pre_commit result. (rx dropped)",
        );
        if let Err(e) = value {
            // receive channel discarding error, log for debugging.
            log_executor_error_occurred(
                e,
                &counters::PIPELINE_DISCARDED_EXECUTOR_ERROR_COUNT,
                block_id,
            );
        }
    }
}
