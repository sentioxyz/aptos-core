// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use anyhow::{bail, ensure, format_err, Result};
use aptos_config::config::{
    RocksdbConfigs, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::AptosDB;
use aptos_storage_interface::{DbReader, MAX_REQUEST_LIMIT};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    state_store::{state_key::StateKey, state_key_prefix::StateKeyPrefix, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use std::sync::Arc;
use std::str::FromStr;
use aptos_framework::natives::code::PackageRegistry;
use aptos_logger::error;
use aptos_rest_client::aptos_api_types::{ResourceGroup, TransactionOnChainData};
use aptos_state_view::TStateView;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_types::access_path::{AccessPath,Path};
use aptos_types::state_store::state_key::StateKeyInner;
use aptos_vm::move_vm_ext::AptosMoveResolver;
use aptos_utils::aptos_try;
use aptos_vm::data_cache::AsMoveResolver;

use move_core_types::language_storage::StructTag;
use crate::sync_tracer_view::AptosTracerInterface;

pub struct DBTracerInterface(Arc<dyn DbReader>);

impl DBTracerInterface {
    pub fn open<P: AsRef<std::path::Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self(Arc::new(AptosDB::open(
            db_root_path,
            true,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfigs::default(),
            false,
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        )?)))
    }
}

impl AptosTracerInterface for DBTracerInterface {
    fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>> {
        let key_prefix = StateKeyPrefix::from(account);
        let mut iter = self
            .0
            .get_prefixed_state_value_iterator(&key_prefix, None, version)?;

        if iter.next().is_some() {
            bail!(
                "Too many state items under state key prefix {:?}.",
                key_prefix
            );
        }

        let mut resource_iter = iter
            .filter_map(|res| match res {
                Ok((k, v)) => match k.inner() {
                    StateKeyInner::AccessPath(AccessPath { address: _, path }) => {
                        match Path::try_from(path.as_slice()) {
                            Ok(Path::Resource(struct_tag)) => {
                                Some(Ok((struct_tag, v.into_bytes())))
                            }
                            // TODO: Consider expanding to Path::Resource
                            Ok(Path::ResourceGroup(struct_tag)) => {
                                Some(Ok((struct_tag, v.into_bytes())))
                            }
                            Ok(Path::Code(_)) => None,
                            Err(e) => Some(Err(anyhow::Error::from(e))),
                        }
                    }
                    _ => {
                        error!("storage prefix scan return inconsistent key ({:?}) with expected key prefix ({:?}).", k, StateKeyPrefix::from(account));
                        Some(Err(format_err!( "storage prefix scan return inconsistent key ({:?})", k )))
                    }
                },
                Err(e) => Some(Err(e)),
            })
            .take(MAX_REQUEST_LIMIT as usize + 1);
        let kvs = resource_iter
            .by_ref()
            .take(MAX_REQUEST_LIMIT as usize)
            .collect::<Result<Vec<(StructTag, Vec<u8>)>>>()?;

        let state_view = self.0.state_view_at_version(Some(version))?;

        // Extract resources from resource groups and flatten into all resources
        let kvs = kvs
            .into_iter()
            .map(|(key, value)| {
                let is_resource_group =
                    |resolver: &dyn AptosMoveResolver, struct_tag: &StructTag| -> bool {
                        aptos_try!({
                            let md = aptos_framework::get_metadata(
                                &resolver.get_module_metadata(&struct_tag.module_id()),
                            )?;
                            md.struct_attributes
                                .get(struct_tag.name.as_ident_str().as_str())?
                                .iter()
                                .find(|attr| attr.is_resource_group())?;
                            Some(())
                        })
                            .is_some()
                    };

                let resolver = state_view.as_move_resolver();
                if is_resource_group(&resolver, &key) {
                    // An error here means a storage invariant has been violated
                    bcs::from_bytes::<ResourceGroup>(&value)
                        .map(|map| {
                            map.into_iter()
                                .map(|(key, value)| (key, value))
                                .collect::<Vec<_>>()
                        })
                        .map_err(|e| e.into())
                } else {
                    Ok(vec![(key, value)])
                }
            })
            .collect::<Result<Vec<Vec<(StructTag, Vec<u8>)>>>>()?
            .into_iter()
            .flatten()
            .map(|(key, value)| (key.access_vector(), value))
            .collect::<BTreeMap<_, _>>();

        Ok(Some(AccountState::new(account, kvs)))
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        let state_view = self.0.state_view_at_version(Some(version))?;
        return state_view.get_state_value(state_key);
    }

    fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        let txn_iter = self.0.get_transaction_iterator(start, limit)?;
        let txn_info_iter = self.0.get_transaction_info_iterator(start, limit)?;
        let txns = txn_iter.collect::<Result<Vec<_>>>()?;
        let txn_infos = txn_info_iter.collect::<Result<Vec<_>>>()?;
        ensure!(txns.len() == txn_infos.len());
        Ok((txns, txn_infos))
    }

    fn get_transaction_by_hash(&self, _hash: String) -> Result<TransactionOnChainData> {
        let ledger_version = self.get_latest_version().unwrap();
        let from_db: Result<Option<TransactionOnChainData>> = self
            .0
            .get_transaction_by_hash(_hash.parse()?, ledger_version, true)?
            .map(|t| -> Result<TransactionOnChainData> {
                // the type is Vec<(Transaction, TransactionOutput)> - given we have one transaction here, there should only ever be one value in this array
                let (_, txn_output) = &self
                    .0
                    .get_transaction_outputs(t.version, 1, t.version)?
                    .transactions_and_outputs[0];
                self.0.get_accumulator_root_hash(t.version)
                    .map(|h| (t, h, txn_output).into())
            })
            .transpose();
        match from_db {
            Ok(v) => {
                match v {
                    Some(v) => Ok(v),
                    None => bail!("Transaction not found")
                }
            }
            Err(e) => {
                bail!(e)
            }
        }
    }

    fn get_latest_version(&self) -> Result<Version> {
        self.0.get_latest_version()
    }

    fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        let ledger_version = self.get_latest_version()?;
        Ok(self
            .0
            .get_account_transaction(account, seq, false, ledger_version)?
            .map(|info| info.version))
    }

    fn get_package_registry(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<PackageRegistry>> {
        let path =
            AccessPath::resource_access_path(account, StructTag::from_str("0x1::code::PackageRegistry").unwrap()).expect("access path in test");
        let state_key = StateKey::access_path(path);
        let state_value = self.get_state_value_by_version(&state_key, version)?;
        match state_value {
            Some(state_value) => {
                let package_registry: PackageRegistry = bcs::from_bytes(&state_value.into_bytes()).unwrap();
                Ok(Some(package_registry))
            }
            None => Ok(None)
        }
    }
}
