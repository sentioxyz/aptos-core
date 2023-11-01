use std::sync::Arc;
// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use anyhow::{anyhow, Result};
use aptos_framework::natives::code::PackageRegistry;
use aptos_state_view::TStateView;
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    account_state::AccountState,
    account_view::AccountView,
    on_chain_config::ValidatorSet,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::{Transaction, TransactionInfo, Version},
};
use move_binary_format::file_format::CompiledModule;
use aptos_rest_client::aptos_api_types::TransactionOnChainData;

// TODO(skedia) Clean up this interfact to remove account specific logic and move to state store
// key-value interface with fine grained storage project
pub trait AptosTracerInterface: Sync {
    fn get_account_state_by_version(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<AccountState>>;

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

    fn get_transaction_by_hash(
        &self,
        hash: String,
    ) -> Result<TransactionOnChainData>;

    fn get_latest_version(&self) -> Result<Version>;

    fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>>;

    fn get_framework_modules_by_version(
        &self,
        version: Version,
    ) -> Result<Vec<CompiledModule>> {
        let mut acc = vec![];
        for module_bytes in self
            .get_account_state_by_version(CORE_CODE_ADDRESS, version)?
            .ok_or_else(|| anyhow!("Failure reading aptos root address state"))?
            .get_modules()
        {
            acc.push(
                CompiledModule::deserialize(module_bytes)
                    .map_err(|e| anyhow!("Failure deserializing module: {:?}", e))?,
            )
        }
        Ok(acc)
    }

    /// Get the account states of the most critical accounts, including:
    /// 1. Aptos Framework code address
    /// 2. All validator addresses
    fn get_admin_accounts(
        &self,
        version: Version,
    ) -> Result<Vec<(AccountAddress, AccountState)>> {
        let mut result = vec![];
        let aptos_framework = self
            .get_account_state_by_version(CORE_CODE_ADDRESS, version)?
            .ok_or_else(|| anyhow!("Aptos framework account doesn't exist"))?;

        // Get all validator accounts
        let validators = aptos_framework
            .get_config::<ValidatorSet>()?
            .ok_or_else(|| anyhow!("validator_config doesn't exist"))?;

        // Get code account
        result.push((
            CORE_CODE_ADDRESS,
            self.get_account_state_by_version(CORE_CODE_ADDRESS, version)?
                .ok_or_else(|| anyhow!("core_code_address doesn't exist"))?,
        ));

        // Get all validator accounts
        for validator_info in validators.payload() {
            let addr = *validator_info.account_address();
            result.push((
                addr,
                self.get_account_state_by_version(addr, version)?
                    .ok_or_else(|| anyhow!("validator {:?} doesn't exist", addr))?,
            ));
        }
        Ok(result)
    }

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
        let bytes_opt = self
            .db
            .get_state_value_by_version(state_key, version)?
            .map(|v| v.into_bytes());
        Ok(bytes_opt.map(StateValue::new_legacy))
    }
}

impl TStateView for SyncTracerView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        self.get_state_value_internal(state_key, self.version)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        unimplemented!()
    }
}
