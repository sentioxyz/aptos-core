// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use aptos_config::config::{RocksdbConfigs, BUFFERED_STATE_TARGET_ITEMS, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG, StorageDirPaths, HotStateConfig};
use aptos_db::AptosDB;
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use std::{collections::HashMap, path::Path, sync::Arc};
use aptos_framework::natives::code::{PackageMetadata, PackageRegistry};
use aptos_rest_client::aptos_api_types::TransactionOnChainData;

use move_core_types::language_storage::ModuleId;
use crate::sync_tracer_view::{AptosTracerInterface, FilterCondition};

pub struct DBTracerInterface(Arc<dyn DbReader>);

impl DBTracerInterface {
    pub fn open<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self(Arc::new(
            AptosDB::open(
                StorageDirPaths::from_path(db_root_path),
                true,
                NO_OP_STORAGE_PRUNER_CONFIG,
                RocksdbConfigs::default(),
                BUFFERED_STATE_TARGET_ITEMS,
                DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
                None,
                HotStateConfig::default()
            )
                .map_err(anyhow::Error::from)?,
        )))
    }
}

impl AptosTracerInterface for DBTracerInterface {
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        self.0
            .get_state_value_by_version(state_key, version)
            .map_err(Into::into)
    }

    fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        let txn_iter = self.0.get_transaction_iterator(start, limit)?;
        let txn_info_iter = self.0.get_transaction_info_iterator(start, limit)?;
        let txns = txn_iter
            .map(|res| res.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        let txn_infos = txn_info_iter
            .map(|res| res.map_err(Into::into))
            .collect::<Result<Vec<_>>>()?;
        ensure!(txns.len() == txn_infos.len());
        Ok((txns, txn_infos))
    }

    fn get_and_filter_committed_transactions(
        &self,
        _start: Version,
        _limit: u64,
        _filter_condition: FilterCondition,
        _package_cache: &mut HashMap<
            ModuleId,
            (
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            ),
        >,
    ) -> Result<
        Vec<(
            u64,
            Transaction,
            Option<(
                AccountAddress,
                String,
                HashMap<(AccountAddress, String), PackageMetadata>,
            )>,
        )>,
    > {
        unimplemented!();
    }

    fn get_transaction_by_hash(&self, _hash: String) -> Result<TransactionOnChainData> {
        todo!()
    }

    fn get_latest_ledger_info_version(&self) -> Result<Version> {
        self.0.get_latest_ledger_info_version().map_err(Into::into)
    }

    fn get_version_by_account_sequence(
        &self,
        _account: AccountAddress,
        _seq: u64,
    ) -> Result<Option<Version>> {
        // TODO: This API was removed from DbReader in upstream. Needs reimplementation if DB mode is needed.
        unimplemented!("get_account_ordered_transaction was removed from DbReader")
    }

    fn get_package_registry(
        &self,
        _account: AccountAddress,
        _version: Version,
    ) -> Result<Option<PackageRegistry>> {
        todo!()
    }
}
