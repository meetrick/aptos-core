use crate::{schema::state_value::StateValueSchema, state_kv_db::StateKvDb, AptosDB};
use aptos_config::config::RocksdbConfig;
use aptos_db_indexer::db_ops::open_internal_indexer_db;
use aptos_db_indexer_schemas::schema::{
    event_by_key::EventByKeySchema, event_by_version::EventByVersionSchema,
    state_keys::StateKeysSchema, transaction_by_account::TransactionByAccountSchema,
};
use aptos_schemadb::{ReadOptions, DB};
use aptos_storage_interface::DbReader;
use aptos_types::{
    contract_event::ContractEvent,
    event::EventKey,
    transaction::{Transaction::UserTransaction, TransactionListWithProof},
};

pub fn validate_db_data() -> Result<()> {
    let target_ledger_version = 1000;
    let db_root_path = "/path/to/db";
    let internal_indexer_db_path = "/path/to/internal_indexer_db";
    println!("Validating db statekeys");
    let state_kv_db = StateKvDb::open(db_root_path, RocksdbConfig::default(), true, true)?;
    let internal_db =
        open_internal_indexer_db(internal_indexer_db_path, &RocksdbConfig::default())?;

    for shard_id in 0..16 {
        let shard = state_kv_db.db_shard(shard_id);
        verify_state_kv(shard, &internal_db)
    }
    println!("Validation events and transactions");
    let aptos_db = AptosDB::new_for_test_with_sharding(db_root_path, 1000000);
    let batch_size = 10_000;
    let start_version = 0;
    while start_version < target_ledger_version {
        let num_of_txns = std::cmp::min(batch_size, target_ledger_version - start_version);
        let txns =
            aptos_db.get_transactions(start_version, num_of_txns, target_ledger_version, true)?;
        verify_transactions(&txns, &internal_db, start_version)?;
        verify_events(&txns, &internal_db, start_version)?;
        assert_eq!(txns.transactions.len() as u64, num_of_txns);
        start_version += txns.transactions.len() as u64;
    }

    Ok(())
}

fn verify_state_kv(shard: &DB, internal_db: &DB) -> Result<()> {
    let mut read_opts = ReadOptions::default();
    let iter = shard.iter_with_opts::<StateValueSchema>(read_opts)?;
    for value in iter {
        let (key, _) = value?;
        let (state_key, version) = key;
        match internal_db.get::<StateKeysSchema>(&state_key) {
            Ok(None) => {
                panic!("State key not found in state keys db: {:?}", state_key);
            },
            Err(e) => {
                panic!("Error while fetching state key: {:?}", e);
            },
            _ => continue,
        }
    }
    Ok(())
}

fn verify_transactions(
    transaction_list: &TransactionListWithProof,
    internal_indexer_db: &DB,
    start_version: u64,
) -> Result<()> {
    for (idx, txn) in transaction_list.transactions.iter().enumerate() {
        match txn {
            UserTransaction(signed_transaction) => {
                let key = (
                    signed_transaction.sender(),
                    signed_transaction.sequence_number(),
                );
                match internal_indexer_db.get::<TransactionByAccountSchema>(&key)? {
                    Some(version) => assert_eq!(version, start_version + idx as u64),
                    None => {
                        panic!("Transaction not found in internal indexer db: {:?}", key);
                    },
                }
            },
            _ => continue,
        }
    }
    Ok(())
}

fn verify_event_by_key(
    event_key: &EventKey,
    seq_num: u64,
    internal_indexer_db: &DB,
    expected_idx: usize,
    expected_version: u64,
) -> Result<()> {
    match internal_indexer_db.get::<EventByKeySchema>(&(event_key.clone(), seq_num)) {
        Ok(None) => {
            panic!("Event not found in internal indexer db: {:?}", event_key);
        },
        Err(e) => {
            panic!("Error while fetching event: {:?}", e);
        },
        Ok(Some((version, idx))) => {
            assert!(idx as usize == expected_idx && version == expected_version);
        },
    }
    Ok(())
}

fn verify_event_by_version(
    event_key: &EventKey,
    seq_num: u64,
    internal_indexer_db: &DB,
    version: u64,
    expected_idx: usize,
) -> Result<()> {
    match internal_indexer_db.get::<EventByVersionSchema>(&(event_key.clone(), version, seq_num)) {
        Ok(None) => {
            panic!("Event not found in internal indexer db: {:?}", event_key);
        },
        Err(e) => {
            panic!("Error while fetching event: {:?}", e);
        },
        Ok(Some(idx)) => {
            assert!(idx as usize == expected_idx);
        },
    }
    Ok(())
}

fn verify_events(
    transaction_list: &TransactionListWithProof,
    internal_indexer_db: &DB,
    start_version: u64,
) -> Result<()> {
    let mut version = start_version;
    match transaction_list.events {
        None => {
            return Ok(());
        },
        Some(event_vec) => {
            for events in event_vec {
                for (idx, event) in events.iter().enumerate() {
                    match event {
                        ContractEvent::V1(event) => {
                            let seq_num = event.sequence_number();
                            let event_key = event.key();
                            verify_event_by_version(
                                event_key,
                                seq_num,
                                internal_indexer_db,
                                version,
                                idx,
                            )?;
                            verify_event_by_key(
                                event_key,
                                seq_num,
                                internal_indexer_db,
                                idx,
                                version,
                            )?;
                        },
                        _ => continue,
                    }
                }
                version += 1;
            }
        },
    }
    Ok(())
}
