// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_rest_client::{Client};
use aptos_types::{
    transaction::{
        Transaction, TransactionPayload,
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
use aptos_vm::transaction_metadata::TransactionMetadata;
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
        txn_hash: String,
    ) -> Result<CallTrace> {
        let txn_data = self.debugger.get_transaction_by_hash(txn_hash).await?;
        let txn = txn_data.transaction;
        let state_view = DebuggerStateView::new(self.debugger.clone(), Version::from(txn_data.version));
        let call_traces = match txn {
            Transaction::UserTransaction(user_txn) => {
                let txn_metadata = TransactionMetadata::new(&user_txn);
                match user_txn.payload() {
                    TransactionPayload::EntryFunction(entry_func) => {
                        AptosVM::get_call_trace(
                            &state_view,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                            entry_func.args().to_vec(),
                            txn_metadata.senders(),
                            user_txn.max_gas_amount(),
                        )
                    },
                    _ => Ok(CallTraces::new()),
                }
            },
            _ => Ok(CallTraces::new()),
        };

        match call_traces {
            Ok(mut _call_traces) => {
                Ok(CallTrace::from(_call_traces.root().unwrap()))
            }
            Err(err) => {
                Err(anyhow!(err))
            }
        }
    }
}
