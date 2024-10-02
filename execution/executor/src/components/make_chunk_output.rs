// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::iter;
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::{CachedStateView, StateCache};
use aptos_types::block_executor::config::BlockExecutorConfigFromOnchain;
use aptos_types::block_executor::partitioner::ExecutableTransactions;
use aptos_types::transaction::signature_verified_transaction::SignatureVerifiedTransaction;
use aptos_vm::VMExecutor;
use anyhow::Result;
use aptos_executor_types::chunk_output::ChunkOutput;
use aptos_executor_types::parsed_transaction_output::TransactionsWithParsedOutput;
use aptos_executor_types::ParsedTransactionOutput;
use aptos_types::transaction::{BlockEndInfo, Transaction, TransactionOutput, TransactionStatus};
use crate::metrics::{update_counters_for_processed_chunk, OTHER_TIMERS};

struct MakeChunkOutput;

impl MakeChunkOutput {
    pub fn by_transaction_execution<V: VMExecutor>(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ChunkOutput> {
        match transactions {
            ExecutableTransactions::Unsharded(txns) => {
                Self::by_transaction_execution_unsharded::<V>(
                    txns,
                    state_view,
                    onchain_config,
                    append_state_checkpoint_to_block,
                )
            },
            ExecutableTransactions::Sharded(txns) => Self::by_transaction_execution_sharded::<V>(
                txns,
                state_view,
                onchain_config,
                append_state_checkpoint_to_block,
            ),
        }
    }

    fn by_transaction_execution_unsharded<V: VMExecutor>(
        transactions: Vec<SignatureVerifiedTransaction>,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ChunkOutput> {
        let block_output = Self::execute_block::<V>(&transactions, &state_view, onchain_config)?;
        let (transaction_outputs, block_end_info) = block_output.into_inner();

        RawChunkOutputParser {
            transactions: transactions.into_iter().map(|t| t.into_inner()).collect(),
            transaction_outputs,
            state_cache,
            block_end_info,
        }
            .parse(append_state_checkpoint_to_block)
    }

    pub fn by_transaction_execution_sharded<V: VMExecutor>(
        transactions: PartitionedTransactions,
        state_view: CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<ChunkOutput> {
        let state_view_arc = Arc::new(state_view);
        let transaction_outputs = Self::execute_block_sharded::<V>(
            transactions.clone(),
            state_view_arc.clone(),
            onchain_config,
        )?;

        // TODO(skedia) add logic to emit counters per shard instead of doing it globally.

        // Unwrapping here is safe because the execution has finished and it is guaranteed that
        // the state view is not used anymore.
        let state_view = Arc::try_unwrap(state_view_arc).unwrap();
        RawChunkOutputParser {
            transactions: PartitionedTransactions::flatten(transactions)
                .into_iter()
                .map(|t| t.into_txn().into_inner())
                .collect(),
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
            block_end_info: None,
        }
            .parse(append_state_checkpoint_to_block)
    }

    pub fn by_transaction_output(
        transactions_and_outputs: Vec<(Transaction, TransactionOutput)>,
        state_view: CachedStateView,
    ) -> Result<Self> {
        let (transactions, transaction_outputs): (Vec<_>, Vec<_>) =
            transactions_and_outputs.into_iter().unzip();

        metrics::update_counters_for_processed_chunk(&transactions, &transaction_outputs, "output");

        // collect all accounts touched and dedup
        let write_set = transaction_outputs
            .iter()
            .map(|o| o.write_set())
            .collect::<Vec<_>>();

        // prime the state cache by fetching all touched accounts
        state_view.prime_cache_by_write_set(write_set)?;

        RawChunkOutputParser {
            transactions,
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
            block_end_info: None,
        }
            .parse(None)
    }

    pub fn apply_to_ledger(
        self,
        base_view: &ExecutedTrees,
        known_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        fail_point!("executor::apply_to_ledger", |_| {
            Err(anyhow::anyhow!("Injected error in apply_to_ledger."))
        });
        ApplyChunkOutput::apply_chunk(self, base_view, known_state_checkpoint_hashes)
    }

    fn execute_block_sharded<V: VMExecutor>(
        partitioned_txns: PartitionedTransactions,
        state_view: Arc<CachedStateView>,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>> {
        if !get_remote_addresses().is_empty() {
            Ok(V::execute_block_sharded(
                REMOTE_SHARDED_BLOCK_EXECUTOR.lock().deref(),
                partitioned_txns,
                state_view,
                onchain_config,
            )?)
        } else {
            Ok(V::execute_block_sharded(
                SHARDED_BLOCK_EXECUTOR.lock().deref(),
                partitioned_txns,
                state_view,
                onchain_config,
            )?)
        }
    }

    /// Executes the block of [Transaction]s using the [VMExecutor] and returns
    /// a vector of [TransactionOutput]s.
    #[cfg(not(feature = "consensus-only-perf-test"))]
    fn execute_block<V: VMExecutor>(
        transactions: &[SignatureVerifiedTransaction],
        state_view: &CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<BlockOutput<TransactionOutput>> {
        Ok(V::execute_block(transactions, state_view, onchain_config)?)
    }

    /// In consensus-only mode, executes the block of [Transaction]s using the
    /// [VMExecutor] only if its a genesis block. In all other cases, this
    /// method returns an [TransactionOutput] with an empty [WriteSet], constant
    /// gas and a [ExecutionStatus::Success] for each of the [Transaction]s.
    #[cfg(feature = "consensus-only-perf-test")]
    fn execute_block<V: VMExecutor>(
        transactions: &[SignatureVerifiedTransaction],
        state_view: &CachedStateView,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<BlockOutput<TransactionOutput>> {
        use aptos_types::{
            state_store::{StateViewId, TStateView},
            transaction::TransactionAuxiliaryData,
            write_set::WriteSet,
        };

        let transaction_outputs = match state_view.id() {
            // this state view ID implies a genesis block in non-test cases.
            StateViewId::Miscellaneous => {
                V::execute_block(transactions, state_view, onchain_config)?
            },
            _ => BlockOutput::new(
                transactions
                    .iter()
                    .map(|_| {
                        TransactionOutput::new(
                            WriteSet::default(),
                            Vec::new(),
                            0, // Keep gas zero to match with StateCheckpoint txn output
                            TransactionStatus::Keep(ExecutionStatus::Success),
                            TransactionAuxiliaryData::None,
                        )
                    })
                    .collect::<Vec<_>>(),
            ),
        };
        Ok(transaction_outputs)
    }

    pub(crate) fn update_counters_for_processed_chunk(&self) {
        for x in [&self.to_commit, &self.to_discard, &self.to_retry] {
            update_counters_for_processed_chunk(x.txns(), x.parsed_outputs(), "execution");
        }
    }
}

struct RawChunkOutputParser {
    /// Input transactions.
    pub transactions: Vec<Transaction>,
    /// Raw VM output.
    pub transaction_outputs: Vec<TransactionOutput>,
    /// Carries the frozen base state view, so all in-mem nodes involved won't drop before the
    /// execution result is processed; as well as all the accounts touched during execution, together
    /// with their proofs.
    pub state_cache: StateCache,
    /// BlockEndInfo outputted by the VM
    pub block_end_info: Option<BlockEndInfo>,
}

impl RawChunkOutputParser {
    fn parse(self, append_state_checkpoint_to_block: Option<HashValue>) -> Result<ChunkOutput> {
        let Self {
            mut transactions,
            transaction_outputs,
            state_cache,
            block_end_info,
        } = self;

        // Parse all outputs.
        let mut transaction_outputs: Vec<ParsedTransactionOutput> = {
            let _timer = OTHER_TIMERS.timer_with(&["parse_output"]);
            transaction_outputs.into_iter().map(Into::into).collect()
        };

        // Isolate retries.
        let to_retry = Self::extract_retries(&mut transactions, &mut transaction_outputs);

        // Collect all statuses.
        let statuses_for_input_txns = {
            let keeps_and_discards = transaction_outputs.iter().map(|t| t.status()).cloned();
            // Forcibly overwriting statuses for retries, since VM can output otherwise.
            let retries = iter::repeat(TransactionStatus::Retry).take(to_retry.len());
            keeps_and_discards.chain(retries).collect()
        };

        // Isolate discards.
        let to_discard = Self::extract_discards(&mut transactions, &mut transaction_outputs);

        // The rest is to be committed, attach block epilogue as needed and optionally get next EpochState.
        let to_commit = TransactionsWithParsedOutput::new(transactions, transaction_outputs);
        let to_commit = Self::maybe_add_block_epilogue(
            to_commit,
            block_end_info.as_ref(),
            append_state_checkpoint_to_block,
        );
        let next_epoch_state = to_commit
            .ends_epoch()
            .then(|| Self::expect_next_epoch_state(&to_commit)?);

        Ok(ChunkOutput {
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
            state_cache,
            block_end_info,
            next_epoch_state,
        })
    }

    fn extract_retries(
        mut transactions: &mut Vec<Transaction>,
        mut transaction_outputs: &mut Vec<ParsedTransactionOutput>,
    ) -> TransactionsWithParsedOutput {
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        let block_gas_limit_marker = transaction_outputs
            .iter()
            .position(|o| matches!(o.status(), TransactionStatus::Retry));

        // Transactions after the epoch ending txn are all to be retried.
        // Transactions after the txn that exceeded per-block gas limit are also to be retried.
        if let Some(pos) = new_epoch_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else if let Some(pos) = block_gas_limit_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else {
            TransactionsWithParsedOutput::new_empty()
        }
    }

    fn extract_discards(
        mut transactions: &mut Vec<Transaction>,
        mut transaction_outputs: &mut Vec<ParsedTransactionOutput>,
    ) -> TransactionsWithParsedOutput {
        let to_discard = {
            let mut res = TransactionsWithParsedOutput::new_empty();
            for idx in 0..transactions.len() {
                if transaction_outputs[idx].status().is_discarded() {
                    res.push(transactions[idx].clone(), transaction_outputs[idx].clone());
                } else if !res.is_empty() {
                    transactions[idx - res.len()] = transactions[idx].clone();
                    transaction_outputs[idx - res.len()] = transaction_outputs[idx].clone();
                }
            }
            if !res.is_empty() {
                let remaining = transactions.len() - res.len();
                transactions.truncate(remaining);
                transaction_outputs.truncate(remaining);
            }
            res
        };

        // Sanity check transactions with the Discard status:
        to_discard.iter().for_each(|(t, o)| {
            // In case a new status other than Retry, Keep and Discard is added:
            if !matches!(o.status(), TransactionStatus::Discard(_)) {
                error!("Status other than Retry, Keep or Discard; Transaction discarded.");
            }
            // VM shouldn't have output anything for discarded transactions, log if it did.
            if !o.write_set().is_empty() || !o.events().is_empty() {
                error!(
                    "Discarded transaction has non-empty write set or events. \
                        Transaction: {:?}. Status: {:?}.",
                    t,
                    o.status(),
                );
                EXECUTOR_ERRORS.inc();
            }
        });

        to_discard
    }

    fn maybe_add_block_epilogue(
        mut to_commit: TransactionsWithParsedOutput,
        block_end_info: Option<&BlockEndInfo>,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> TransactionsWithParsedOutput {
        if !to_commit.ends_epoch() {
            // Append the StateCheckpoint transaction to the end
            if let Some(block_id) = append_state_checkpoint_to_block {
                let state_checkpoint_txn = block_end_info.cloned().map_or(
                    Transaction::StateCheckpoint(block_id),
                    |block_end_info| {
                        Transaction::BlockEpilogue(BlockEpiloguePayload::V0 {
                            block_id,
                            block_end_info,
                        })
                    },
                );
                let state_checkpoint_txn_output: ParsedTransactionOutput =
                    Into::into(TransactionOutput::new(
                        WriteSet::default(),
                        Vec::new(),
                        0,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                        TransactionAuxiliaryData::default(),
                    ));
                to_commit.push(state_checkpoint_txn, state_checkpoint_txn_output);
            }
        }; // else: not adding block epilogue at epoch ending.

        to_commit
    }

    fn expect_next_epoch_state(to_commit: &TransactionsWithParsedOutput) -> Result<EpochState> {
        let last_write_set = to_commit
            .parsed_outputs()
            .last()
            .ok_or_else(|| Err(anyhow!("to_commit is empty.")))?
            .write_set();
        let write_set_view = WriteSetStateView {
            write_set: last_write_set,
        };

        let validator_set = ValidatorSet::fetch_config(&write_set_view)
            .ok_or_else(|| anyhow!("ValidatorSet not touched on epoch change"))?;
        let configuration = ConfigurationResource::fetch_config(&write_set_view)
            .ok_or_else(|| anyhow!("Configuration resource not touched on epoch change"))?;

        Ok(EpochState {
            epoch: configuration.epoch(),
            verifier: (&validator_set).into(),
        })
    }
}

struct WriteSetStateView<'a> {
    write_set: &'a WriteSet,
}

impl<'a> TStateView for WriteSetStateView<'a> {
    type Key = StateKey;

    fn get_state_value(
        &self,
        state_key: &Self::Key,
    ) -> aptos_types::state_store::Result<Option<StateValue>> {
        Ok(self
            .write_set
            .get(state_key)
            .flat_map(|write_op| write_op.as_state_value()))
    }

    fn get_usage(&self) -> aptos_types::state_store::Result<StateStorageUsage> {
        unreachable!("Not supposed to be called on WriteSetStateView.")
    }
}
