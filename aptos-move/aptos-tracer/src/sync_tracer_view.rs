// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_framework::natives::code::{PackageMetadata, PackageRegistry};
use aptos_rest_client::aptos_api_types::TransactionOnChainData;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewId, TStateView, errors::StateViewError,
    },
    transaction::{Transaction, TransactionInfo, Version},
};
use move_core_types::language_storage::ModuleId;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

#[derive(Clone, Copy)]
pub struct FilterCondition {
    pub skip_failed_txns: bool,
    pub skip_publish_txns: bool,
    pub check_source_code: bool,
    pub target_account: Option<AccountAddress>,
}

// TODO(skedia) Clean up this interfact to remove account specific logic and move to state store
// key-value interface with fine grained storage project
pub trait AptosTracerInterface: Sync {
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>>;

    fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)>;

    fn get_and_filter_committed_transactions(
        &self,
        start: Version,
        limit: u64,
        filter_condition: FilterCondition,
        package_cache: &mut HashMap<
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
    >;

    fn get_transaction_by_hash(
        &self,
        hash: String,
    ) -> Result<TransactionOnChainData>;

    fn get_latest_ledger_info_version(&self) -> Result<Version>;

    fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>>;

    fn get_package_registry(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<PackageRegistry>>;
}

pub struct SyncTracerView {
    db: Arc<dyn AptosTracerInterface + Send>,
    version: Version,
}

impl SyncTracerView {
    pub fn new(db: Arc<dyn AptosTracerInterface + Send>, version: Version) -> Self {
        Self { db, version }
    }

    fn get_state_value_internal(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        self.db.get_state_value_by_version(state_key, version)
    }
}

impl TStateView for SyncTracerView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn get_state_value(&self, state_key: &StateKey) -> std::result::Result<Option<StateValue>, StateViewError> {
        self.get_state_value_internal(state_key, self.version)
            .map_err(|e| StateViewError::Other(format!("{}", e)))
    }

    fn get_usage(&self) -> std::result::Result<StateStorageUsage, StateViewError> {
        unimplemented!()
    }
}
