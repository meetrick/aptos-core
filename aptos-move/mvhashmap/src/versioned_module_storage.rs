// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{ShiftedTxnIndex, StorageVersion, TxnIndex};
use aptos_types::{executable::ModulePath, vm::modules::ModuleStorageEntryInterface};
use claims::{assert_none, assert_ok, assert_some};
use crossbeam::utils::CachePadded;
use dashmap::DashMap;
use derivative::Derivative;
use move_binary_format::errors::VMResult;
use std::{collections::BTreeMap, fmt::Debug, hash::Hash, sync::Arc};
use std::collections::HashMap;
use aptos_crypto::_once_cell::sync::{Lazy, OnceCell};
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::StateView;
use aptos_types::vm::modules::ModuleStorageEntry;
use aptos_vm_types::module_and_script_storage::AsAptosCodeStorage;
use move_core_types::account_address::AccountAddress;
use move_core_types::ident_str;
use move_vm_runtime::{ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment};

static MODULE_CACHE: OnceCell<Option<HashMap<StateKey, Arc<ModuleStorageEntry>>>> = OnceCell::new();

pub fn initialize_module_cache(state_view: &impl StateView, runtime_environment: &RuntimeEnvironment) {
    if MODULE_CACHE.get().is_some() {
        return;
    }

    let module_storage = state_view.as_aptos_code_storage(runtime_environment);
    let ordered_module_names = [
        // Move stdlib.
        ident_str!("vector"),
        ident_str!("signer"),
        ident_str!("error"),
        ident_str!("hash"),
        ident_str!("features"),
        ident_str!("bcs"),
        ident_str!("option"),
        ident_str!("string"),
        ident_str!("fixed_point32"),

        // Aptos stdlib.
        ident_str!("type_info"),
        ident_str!("ed25519"),
        ident_str!("from_bcs"),
        ident_str!("multi_ed25519"),
        ident_str!("table"),
        ident_str!("bls12381"),
        ident_str!("math64"),
        ident_str!("fixed_point64"),
        ident_str!("math128"),
        ident_str!("math_fixed64"),
        ident_str!("table_with_length"),
        ident_str!("copyable_any"),
        ident_str!("simple_map"),
        ident_str!("bn254_algebra"),
        ident_str!("crypto_algebra"),
        ident_str!("aptos_hash"),

        // Framework.
        ident_str!("guid"),
        ident_str!("system_addresses"),
        ident_str!("chain_id"),
        ident_str!("timestamp"),
        ident_str!("event"),
        ident_str!("create_signer"),
        ident_str!("account"),
        ident_str!("aggregator"),
        ident_str!("aggregator_factory"),
        ident_str!("optional_aggregator"),
        ident_str!("transaction_context"),
        ident_str!("randomness"),
        ident_str!("object"),
        ident_str!("aggregator_v2"),
        ident_str!("function_info"),
        ident_str!("fungible_asset"),
        ident_str!("dispatchable_fungible_asset"),
        ident_str!("primary_fungible_store"),
        ident_str!("coin"),
        ident_str!("aptos_coin"),
        ident_str!("aptos_account"),
        ident_str!("chain_status"),
        ident_str!("staking_config"),
        ident_str!("stake"),
        ident_str!("transaction_fee"),
        ident_str!("transaction_validation"),
        ident_str!("reconfiguration_state"),
        ident_str!("state_storage"),
        ident_str!("storage_gas"),
        ident_str!("reconfiguration"),
        ident_str!("config_buffer"),
        ident_str!("randomness_api_v0_config"),
        ident_str!("randomness_config"),
        ident_str!("randomness_config_seqnum"),
        ident_str!("keyless_account"),
        ident_str!("consensus_config"),
        ident_str!("execution_config"),
        ident_str!("validator_consensus_info"),
        ident_str!("dkg"),
        ident_str!("gas_schedule"),
        ident_str!("util"),
        ident_str!("gas_schedule"),
        ident_str!("jwk_consensus_config"),
        ident_str!("jwks"),
        ident_str!("reconfiguration_with_dkg"),
        ident_str!("block"),
        ident_str!("code"),
    ];

    let mut framework = HashMap::new();
    for module_name in ordered_module_names {
        let state_key = StateKey::module(&AccountAddress::ONE, module_name);
        let state_value = assert_ok!(state_view.get_state_value(&state_key));
        let module = assert_ok!(module_storage.fetch_verified_module(&AccountAddress::ONE, module_name));
        if let (Some(state_value), Some(module)) = (state_value, module) {
            let entry = ModuleStorageEntry::from_state_value_and_verified_module(state_value, module);
            framework.insert(state_key, Arc::new(entry));
        }
    }

    MODULE_CACHE.set(Some(framework)).unwrap();
}

pub(crate) fn get_cached<K: ModulePath>(key: &K) -> Option<Arc<ModuleStorageEntry>> {
    let cache = MODULE_CACHE.get().unwrap().as_ref().unwrap();
    cache.get(key.as_state_key()).cloned()
}

/// Represents a version of a module - either written by some transaction, or fetched from storage.
pub type ModuleVersion = Result<TxnIndex, StorageVersion>;

/// Result of a read query on the versioned module storage.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug, PartialEq)]
pub enum ModuleStorageRead {
    /// An existing module at certain index of committed transaction or from the base storage.
    Versioned(
        ModuleVersion,
        #[derivative(PartialEq = "ignore", Debug = "ignore")] Arc<ModuleStorageEntry>,
    ),
    /// If module is not found in storage.
    DoesNotExist,
}

impl ModuleStorageRead {
    pub fn storage_version(entry: Arc<ModuleStorageEntry>) -> Self {
        Self::Versioned(Err(StorageVersion), entry)
    }

    pub fn before_txn_idx(txn_idx: TxnIndex, entry: Arc<ModuleStorageEntry>) -> Self {
        let version = if txn_idx > 0 {
            Ok(txn_idx - 1)
        } else {
            Err(StorageVersion)
        };
        Self::Versioned(version, entry)
    }

    /// If the entry exists, returns it together with its index. Otherwise, returns [None].
    pub fn into_versioned(self) -> Option<(ModuleVersion, Arc<ModuleStorageEntry>)> {
        match self {
            Self::Versioned(version, entry) => Some((version, entry)),
            Self::DoesNotExist => None,
        }
    }
}

/// Represents different versions of module storage information for different transaction indices
/// (including the base storage version).
struct VersionedEntry {
    versions: BTreeMap<ShiftedTxnIndex, CachePadded<Option<Arc<ModuleStorageEntry>>>>,
}

impl VersionedEntry {
    /// A new versioned entry with no written versions yet.
    fn empty() -> Self {
        Self {
            versions: BTreeMap::new(),
        }
    }

    /// Returns the "latest" module entry under the specified index. If such an entry does not
    /// exist, [None] is returned.
    fn get(&self, txn_idx: TxnIndex) -> Option<ModuleStorageRead> {
        use ModuleStorageRead::*;

        self.versions
            .range(ShiftedTxnIndex::zero_idx()..ShiftedTxnIndex::new(txn_idx))
            .next_back()
            .map(|(idx, entry)| match entry.as_ref() {
                Some(entry) => Versioned(idx.idx(), entry.clone()),
                None => DoesNotExist,
            })
    }

    #[cfg(test)]
    fn insert(&mut self, txn_idx: TxnIndex, entry: Option<Arc<ModuleStorageEntry>>) {
        self.versions
            .insert(ShiftedTxnIndex::new(txn_idx), CachePadded::new(entry));
    }
}

/// Module storage, versioned so that we can keep track of module writes of each transaction. In
/// particular, for each key we keep track the writes of all transactions (see [VersionedEntry]).
pub struct VersionedModuleStorage<K> {
    entries: DashMap<K, VersionedEntry>,
}

impl<K: Debug + Hash + Clone + Eq + ModulePath>
    VersionedModuleStorage<K>
{
    /// Returns a new empty versioned module storage.
    pub(crate) fn empty() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Returns the module entry from the module storage. If the entry does not exist,
    /// [ModuleStorageRead::DoesNotExist] is returned. If there is a pending code publish below
    /// the queried index, again the same [ModuleStorageRead::DoesNotExist] is returned as all
    /// pending publishes are treated as non-existent modules.
    pub fn get(&self, key: &K, txn_idx: TxnIndex) -> ModuleStorageRead {
        self.get_impl(key, txn_idx)
            .unwrap_or(ModuleStorageRead::DoesNotExist)
    }

    /// Similar to [VersionedModuleStorage::get]. The difference is that if the module does not
    /// exist in module storage, the passed closure is used to initialize it. In contrast,
    /// [VersionedModuleStorage::get] returns [ModuleStorageRead::DoesNotExist].
    pub fn get_or_else<F>(
        &self,
        key: &K,
        txn_idx: TxnIndex,
        init_func: F,
    ) -> VMResult<ModuleStorageRead>
    where
        F: Fn() -> VMResult<Option<Arc<ModuleStorageEntry>>>,
    {
        if let Some(read) = self.get_impl(key, txn_idx) {
            return Ok(read);
        }

        // Here the versioned map is locked, to ensure a single thread is used to initialize the
        // storage version.
        let mut v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);

        let maybe_entry = init_func()?;
        v.versions.insert(
            ShiftedTxnIndex::zero_idx(),
            CachePadded::new(maybe_entry.clone()),
        );
        drop(v);

        Ok(match maybe_entry {
            Some(e) => ModuleStorageRead::storage_version(e),
            None => ModuleStorageRead::DoesNotExist,
        })
    }

    /// Removes an existing entry at a given index.
    pub fn remove(&self, key: &K, txn_idx: TxnIndex) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry should always exist before removal");
        let removed = versioned_entry
            .versions
            .remove(&ShiftedTxnIndex::new(txn_idx));
        assert_some!(removed, "Entry should always exist before removal");
    }

    /// Marks an entry in module storage as "pending", i.e., yet to be published. The
    /// implementation simply treats pending writes as non-existent modules, so that transactions
    /// with higher indices observe non-existent modules and deterministically fail with a
    /// non-speculative error.
    pub fn write_pending(&self, key: K, txn_idx: TxnIndex) {
        let mut v = self
            .entries
            .entry(key.clone())
            .or_insert_with(VersionedEntry::empty);
        v.versions
            .insert(ShiftedTxnIndex::new(txn_idx), CachePadded::new(None));
    }

    /// Writes a published module to the storage, which is also visible for the transactions with
    /// higher indices.
    pub fn write_published(&self, key: &K, idx_to_publish: TxnIndex, entry: ModuleStorageEntry) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before publishing");

        let prev = versioned_entry.versions.insert(
            ShiftedTxnIndex::new(idx_to_publish),
            CachePadded::new(Some(Arc::new(entry))),
        );
        let prev = assert_some!(prev);
        assert_none!(prev.as_ref());
    }

    /// Write the new module storage entry to the specified key-index pair unless the existing
    /// entry has been already verified. Note that the index at which the modules are verified must
    /// always be the index of a committed transaction.
    pub fn write_if_not_verified(&self, key: &K, version: ModuleVersion, entry: Arc<ModuleStorageEntry>) {
        let mut versioned_entry = self
            .entries
            .get_mut(key)
            .expect("Versioned entry must always exist before it is set as verified");

        let committed_idx = version
            .map(ShiftedTxnIndex::new)
            .unwrap_or(ShiftedTxnIndex::zero_idx());
        let prev_entry = versioned_entry
            .versions
            .get(&committed_idx)
            .expect("At least the base storage version must exist")
            .as_ref()
            .expect("Entry must exist before it is marked as verified");
        if !prev_entry.is_verified() {
            versioned_entry
                .versions
                .insert(committed_idx, CachePadded::new(Some(entry)));
        }
    }

    /// Returns the module entry if it exists in multi-version data structure. If not, [None] is
    /// returned.
    #[inline]
    fn get_impl(&self, key: &K, txn_idx: TxnIndex) -> Option<ModuleStorageRead> {
        if let Some(entry) = get_cached(key) {
            return Some(ModuleStorageRead::Versioned(Err(StorageVersion), entry));
        }

        match self.entries.get(key) {
            Some(v) => v.get(txn_idx),
            None => None,
        }
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use aptos_types::state_store::{
//         state_key::StateKey,
//         state_value::{StateValue, StateValueMetadata},
//     };
//     use bytes::Bytes;
//     use claims::{assert_err_eq, assert_matches, assert_none, assert_ok};
//     use move_binary_format::errors::{Location, PartialVMError};
//     use move_core_types::{metadata::Metadata, vm_status::StatusCode};
//     use move_vm_runtime::RuntimeEnvironment;
//
//     #[derive(Debug, Eq, PartialEq)]
//     struct TestEntry {
//         is_verified: bool,
//         id: usize,
//     }
//
//     impl TestEntry {
//         fn new(is_verified: bool, id: usize) -> Self {
//             Self { is_verified, id }
//         }
//     }
//
//     impl ModuleStorageEntryInterface for TestEntry {
//         fn from_state_value(
//             _runtime_environment: &RuntimeEnvironment,
//             _state_value: StateValue,
//         ) -> VMResult<Self> {
//             unreachable!()
//         }
//
//         fn bytes(&self) -> &Bytes {
//             unreachable!()
//         }
//
//         fn state_value_metadata(&self) -> &StateValueMetadata {
//             unreachable!()
//         }
//
//         fn hash(&self) -> &[u8; 32] {
//             unreachable!()
//         }
//
//         fn metadata(&self) -> &[Metadata] {
//             unreachable!()
//         }
//
//         fn is_verified(&self) -> bool {
//             self.is_verified
//         }
//     }
//
//     fn state_key(name: &str) -> StateKey {
//         StateKey::raw(name.as_bytes())
//     }
//
//     #[test]
//     fn test_module_storage_reads() {
//         let entry = TestEntry::new(false, 0);
//         let read = ModuleStorageRead::storage_version(Arc::new(entry));
//         let (version, _) = assert_some!(read.into_versioned());
//         assert_err_eq!(version, StorageVersion);
//
//         let read: ModuleStorageRead<TestEntry> = ModuleStorageRead::DoesNotExist;
//         assert_none!(read.into_versioned());
//     }
//
//     #[test]
//     fn test_versioned_entry() {
//         // Entry is empty.
//         let mut versioned_entry = VersionedEntry::empty();
//         assert_none!(versioned_entry.get(5));
//
//         versioned_entry.insert(5, Some(Arc::new(TestEntry::new(false, 0))));
//
//         // Entry is only visible to higher-index transactions, so transaction 5 still does not
//         // find anything, but transaction 10 does.
//         assert_none!(versioned_entry.get(5));
//         let read = assert_some!(versioned_entry.get(10));
//         assert_matches!(read, ModuleStorageRead::Versioned(..));
//
//         versioned_entry.insert(6, None);
//
//         // A new pending entry inserted by transaction 6. Transaction 6 should still see the old
//         // version, but transaction 10 sees the new pending entry.
//         let read = assert_some!(versioned_entry.get(6));
//         assert_matches!(read, ModuleStorageRead::Versioned(..));
//
//         let read = assert_some!(versioned_entry.get(10));
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//
//         assert_none!(versioned_entry.get(5));
//     }
//
//     #[test]
//     fn test_versioned_module_storage_1() {
//         let module_storage: VersionedModuleStorage<StateKey, TestEntry> =
//             VersionedModuleStorage::empty();
//
//         // Non-existing entries should not be found.
//         let read = module_storage.get(&state_key("0x1::foo"), 10);
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//
//         // Even if we set the base value, if it does not exist, we still observe that the entry
//         // does not exist.
//         let result = module_storage.get_or_else(&state_key("0x1::bar"), 10, || Ok(None));
//         assert_matches!(assert_ok!(result), ModuleStorageRead::DoesNotExist);
//
//         // If the entry exists, it must be cached at storage version.
//         let result = module_storage.get_or_else(&state_key("0x1::foo"), 10, || {
//             Ok(Some(Arc::new(TestEntry::new(false, 0))))
//         });
//         assert_matches!(
//             assert_ok!(result),
//             ModuleStorageRead::Versioned(Err(StorageVersion), ..)
//         );
//
//         // Errors from setting the base values are propagated.
//         let err = PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined);
//         let result = module_storage.get_or_else(&state_key("0x1::baz"), 10, || Err(err.clone()));
//         assert_err_eq!(result, err);
//     }
//
//     #[test]
//     fn test_versioned_module_storage_2() {
//         let module_storage: VersionedModuleStorage<StateKey, TestEntry> =
//             VersionedModuleStorage::empty();
//
//         // Set some base values:
//         //   - 0x1::foo does not exist
//         //   - 0x1::bar exists
//         assert_ok!(module_storage.get_or_else(&state_key("0x1::foo"), 10, || Ok(None)));
//         assert_ok!(
//             module_storage.get_or_else(&state_key("0x1::bar"), 10, || Ok(Some(Arc::new(
//                 TestEntry::new(false, 0)
//             ))))
//         );
//
//         let read = module_storage.get(&state_key("0x1::foo"), 10);
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//         let read = module_storage.get(&state_key("0x1::bar"), 10);
//         assert_matches!(read, ModuleStorageRead::Versioned(Err(StorageVersion), ..));
//
//         // Transactions 5, 6, and 7 add pending writes at 0x1::foo, 0x1::bar and 0x1::buz.
//         module_storage.write_pending(state_key("0x1::foo"), 5);
//         module_storage.write_pending(state_key("0x1::bar"), 6);
//         module_storage.write_pending(state_key("0x1::buz"), 7);
//
//         // Transaction 10 still does not see 0x1::foo and 0x1::buz.
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::foo"), 10, || unreachable!()));
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::buz"), 10, || unreachable!()));
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//
//         // 0x1::bar storage version is only visible for transactions < 7.
//         let read = module_storage.get(&state_key("0x1::bar"), 7);
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//         let read = module_storage.get(&state_key("0x1::bar"), 6);
//         assert_matches!(read, ModuleStorageRead::Versioned(Err(StorageVersion), ..));
//
//         // Transaction 6 removes its writes, and so 0x1::bar becomes visible for all transactions.
//         module_storage.remove(&state_key("0x1::bar"), 6);
//
//         let read = module_storage.get(&state_key("0x1::bar"), 7);
//         assert_matches!(read, ModuleStorageRead::Versioned(Err(StorageVersion), ..));
//         let read = module_storage.get(&state_key("0x1::bar"), 6);
//         assert_matches!(read, ModuleStorageRead::Versioned(Err(StorageVersion), ..));
//
//         // Now transactions 5 and 7 actually publish.
//         module_storage.write_published(&state_key("0x1::foo"), 5, TestEntry::new(false, 1));
//         module_storage.write_published(&state_key("0x1::buz"), 7, TestEntry::new(false, 2));
//
//         // Transactions below see no change at all.
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::foo"), 5, || unreachable!()));
//         assert_matches!(read, ModuleStorageRead::DoesNotExist);
//         let read = assert_ok!(
//             module_storage.get_or_else(&state_key("0x1::buz"), 4, || Ok(Some(Arc::new(
//                 TestEntry::new(false, 3)
//             ))))
//         );
//         assert_matches!(read, ModuleStorageRead::Versioned(Err(StorageVersion), ..));
//
//         // Transactions above see the new version.
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::foo"), 10, || unreachable!()));
//         assert_matches!(read, ModuleStorageRead::Versioned(Ok(5), ..));
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::buz"), 10, || unreachable!()));
//         assert_matches!(read, ModuleStorageRead::Versioned(Ok(7), ..));
//     }
//
//     #[test]
//     fn test_versioned_module_storage_3() {
//         let module_storage: VersionedModuleStorage<StateKey, TestEntry> =
//             VersionedModuleStorage::empty();
//
//         assert_ok!(
//             module_storage.get_or_else(&state_key("0x1::foo"), 6, || Ok(Some(Arc::new(
//                 TestEntry::new(false, 0)
//             ))))
//         );
//
//         module_storage.write_pending(state_key("0x1::bar"), 6);
//         module_storage.write_published(&state_key("0x1::bar"), 6, TestEntry::new(false, 1));
//
//         // Module storage stores two modules, 0x1::foo at storage version, 0x1::bar published by
//         // transaction 6.
//
//         // Store verified modules.
//         module_storage.write_if_not_verified(
//             &state_key("0x1::foo"),
//             Err(StorageVersion),
//             Arc::new(TestEntry::new(true, 2)),
//         );
//         module_storage.write_if_not_verified(
//             &state_key("0x1::bar"),
//             Ok(6),
//             Arc::new(TestEntry::new(true, 3)),
//         );
//
//         // Any other attempts to store the same module will live the module storage intact.
//         module_storage.write_if_not_verified(
//             &state_key("0x1::foo"),
//             Err(StorageVersion),
//             Arc::new(TestEntry::new(true, 4)),
//         );
//         module_storage.write_if_not_verified(
//             &state_key("0x1::bar"),
//             Ok(6),
//             Arc::new(TestEntry::new(true, 5)),
//         );
//
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::foo"), 10, || unreachable!()));
//         if let ModuleStorageRead::Versioned(Err(StorageVersion), e) = read {
//             assert_eq!(e.id, 2);
//         } else {
//             panic!("0x1::foo must exist at storage version")
//         }
//
//         let read =
//             assert_ok!(module_storage.get_or_else(&state_key("0x1::bar"), 10, || unreachable!()));
//         if let ModuleStorageRead::Versioned(Ok(6), e) = read {
//             assert_eq!(e.id, 3);
//         } else {
//             panic!("0x1::foo must exist at version 6")
//         }
//     }
// }