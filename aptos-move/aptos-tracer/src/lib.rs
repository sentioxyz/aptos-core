// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result};
use aptos_rest_client::Client;
use aptos_types::{
    transaction::{
        Transaction, TransactionInfo, TransactionPayload,
        Version,
    },
};
use aptos_validator_interface::{
    AptosValidatorInterface, DBDebuggerInterface, DebuggerStateView, RestDebuggerInterface,
};
use aptos_vm::{
    AptosVM
};
use std::{path::Path, sync::Arc};
use aptos_rest_client::aptos_api_types::call_trace::CallTrace;
use move_core_types::call_trace::CallTraces;

pub struct AptosTracer {
    debugger: Arc<dyn AptosValidatorInterface + Send>,
}

impl AptosTracer {
    pub fn new(debugger: Arc<dyn AptosValidatorInterface + Send>) -> Self {
        Self { debugger }
    }

    pub fn rest_client(rest_client: Client) -> Result<Self> {
        Ok(Self::new(Arc::new(RestDebuggerInterface::new(rest_client))))
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self::new(Arc::new(DBDebuggerInterface::open(
            db_root_path,
        )?)))
    }

    pub async fn trace_transaction(
        &self,
        mut begin: Version,
        mut limit: u64,
        txn_hash: String,
    ) -> Result<CallTrace> {
        let (mut txns, mut txn_infos) = self
            .debugger
            .get_committed_transactions(begin, limit)
            .await?;

        let txn_info_with_index = txn_infos.into_iter().enumerate().find(|(index, t)| {
            match t {
                TransactionInfo::V0(info) => {
                    let recorded = info.transaction_hash().to_hex_literal();
                    recorded == txn_hash
                }
            }
        });
        let (index, txn_info) = txn_info_with_index.unwrap();
        let txn = txns.get(index);
        let state_view = DebuggerStateView::new(self.debugger.clone(), begin);
        let call_traces = match txn {
            None => Ok(CallTraces::new()),
            Some(txn) => {
                match txn {
                    Transaction::UserTransaction(user_txn) => {
                        match user_txn.payload() {
                            TransactionPayload::EntryFunction(entry_func) => {
                                AptosVM::get_call_trace(
                                    &state_view,
                                    entry_func.module().clone(),
                                    entry_func.function().to_owned(),
                                    entry_func.ty_args().to_vec(),
                                    entry_func.args().to_vec(),
                                    txn_info.gas_used(),
                                )
                            },
                            _ => Ok(CallTraces::new()),
                        }
                    },
                    _ => Ok(CallTraces::new()),
                }
            }
        };

        match call_traces {
            Ok(mut _call_traces) => {
                Ok(CallTrace::from(_call_traces.root().unwrap()))
            }
            Err(_) => {
                Ok(CallTrace {
                    pc: 0,
                    module_id: "".to_string(),
                    func_name: "".to_string(),
                    inputs: vec![],
                    outputs: vec![],
                    type_args: vec![],
                    sub_traces: vec![],
                })
            }
        }
    }
}
