// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use anyhow::{anyhow, Result};
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{Transaction, TransactionInfo, Version},
};
use aptos_framework::natives::code::PackageRegistry;
use aptos_rest_client::aptos_api_types::{AptosError, AptosErrorCode, HashValue, TransactionData, TransactionOnChainData};
use aptos_rest_client::Client;
use aptos_rest_client::error::{AptosErrorResponse, RestError};
use move_core_types::language_storage::ModuleId;
use aptos_framework::natives::code::PackageMetadata;
use crate::sync_tracer_view::{AptosTracerInterface, FilterCondition};

pub struct RestTracerInterface{
    client: Client,
}

impl RestTracerInterface {
    pub fn new(client: Client) -> Self {
        Self {
            client,
        }
    }
}

impl AptosTracerInterface for RestTracerInterface {
    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<StateValue>> {
        let client = self.client.clone();
        let state_key = state_key.clone();

        match std::thread::spawn(move ||
                client.get_raw_state_value_sync(&state_key, version)
        ).join().unwrap(){
            Ok(resp) => Ok(Some(bcs::from_bytes(&resp.into_inner())?)),
            Err(err) => match err {
                RestError::Api(AptosErrorResponse {
                                   error:
                                   AptosError {
                                       error_code: AptosErrorCode::StateValueNotFound,
                                       ..
                                   },
                                   ..
                               }) => Ok(None),
                _ => Err(anyhow!(err)),
            },
        }
    }

    fn get_committed_transactions(
        &self,
        start: Version,
        limit: u64,
    ) -> Result<(Vec<Transaction>, Vec<TransactionInfo>)> {
        let client = self.client.clone();

        let txnsAndInfos = std::thread::spawn(move || {
            let mut txns = Vec::with_capacity(limit as usize);
            let mut txn_infos = Vec::with_capacity(limit as usize);
            while txns.len() < limit as usize {
                client
                    .get_transactions_bcs_sync(
                        Some(start + txns.len() as u64),
                        Some(limit as u16 - txns.len() as u16),
                    )
                    .unwrap()
                    .into_inner()
                    .into_iter()
                    .for_each(|txn| {
                        txns.push(txn.transaction);
                        txn_infos.push(txn.info);
                    });
                println!("Got {}/{} txns from RestApi.", txns.len(), limit);
            }
            (txns, txn_infos)
        }).join().unwrap();

        Ok((txnsAndInfos.0, txnsAndInfos.1))
    }

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
    > {
        todo!();
    }

    fn get_transaction_by_hash(&self, _hash: String) -> Result<TransactionOnChainData> {
        let client = self.client.clone();

        let resp = std::thread::spawn(move || {
            client.get_transaction_by_hash_bcs_sync(HashValue::from_str(_hash.as_str()).unwrap().0)
        }).join().unwrap();

        match resp {
            Ok(resp) => {
                match resp.into_inner() {
                    TransactionData::OnChain(data) => {
                        Ok(data)
                    }
                    TransactionData::Pending(_) => {
                        Err(anyhow!("Transaction is in pending status"))
                    }
                }
            }
            Err(err) => match err {
                RestError::Api(AptosErrorResponse {
                                    error:
                                    AptosError {
                                        error_code: AptosErrorCode::StateValueNotFound,
                                        ..
                                    },
                                    ..
                                }) => Err(anyhow!(err)),
                _ => Err(anyhow!(err)),
            },
        }
    }

    fn get_latest_ledger_info_version(&self) -> Result<Version> {
        let client = self.client.clone();

        std::thread::spawn(move || {
            Ok(client.get_ledger_information_sync()?.into_inner().version)
        }).join().unwrap()
    }

    fn get_version_by_account_sequence(
        &self,
        account: AccountAddress,
        seq: u64,
    ) -> Result<Option<Version>> {
        let client = self.client.clone();

        std::thread::spawn(move || {
            Ok(Some(
                    client
                        .get_account_transactions_bcs_sync(account, Some(seq), None)?
                        .into_inner()[0]
                        .version,
                ))
        }).join().unwrap()
    }

    fn get_package_registry(
        &self,
        account: AccountAddress,
        version: Version,
    ) -> Result<Option<PackageRegistry>> {
        let client = self.client.clone();

        std::thread::spawn(move || {
                Ok(Some(
                    client
                        .get_account_resource_at_version_bcs_sync::<PackageRegistry>(account, "0x1::code::PackageRegistry", version)?
                        .into_inner()
                ))
        }).join().unwrap()
    }
}
