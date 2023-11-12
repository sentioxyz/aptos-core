// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod config;
mod server;
mod sync_tracer_view;
mod sync_storage_interface;

pub use server::run_debugger_server;
pub use config::DebuggerServerConfig;

use anyhow::{Result};
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
use std::collections::HashMap;
use std::str::FromStr;
use codespan::Files;
use serde::{Deserialize, Serialize};
use aptos_framework::natives::code::PackageRegistry;
use aptos_framework::{unzip_metadata, unzip_metadata_str};

use aptos_vm::transaction_metadata::TransactionMetadata;
use move_binary_format::file_format::{CodeOffset, FunctionDefinitionIndex, TableIndex};
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::account_address::AccountAddress;
use move_core_types::call_trace::{InternalCallTrace};
use crate::sync_storage_interface::DBTracerInterface;
use crate::sync_tracer_view::{AptosTracerInterface, SyncTracerView};

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
    ) -> Result<CallTraceWithSource> {
        let txn_data = self.debugger.get_transaction_by_hash(txn_hash).await?;
        let txn = txn_data.transaction;
        let state_view = DebuggerStateView::new(self.debugger.clone(), Version::from(txn_data.version));
        let call_traces = match txn {
            Transaction::UserTransaction(user_txn) => {
                let txn_metadata = TransactionMetadata::new(&user_txn);
                match user_txn.payload() {
                    TransactionPayload::EntryFunction(entry_func) => {
                        let call_trace = AptosVM::get_call_trace(
                            &state_view,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                            entry_func.args().to_vec(),
                            txn_metadata.senders(),
                            user_txn.max_gas_amount(),
                        ).unwrap();
                        // get all the package registries from accounts in call_trace
                        let mut package_registries = HashMap::new();
                        for account in &call_trace.1 {
                            let package_registry = self.debugger.get_package_registry(
                                AccountAddress::from_str(account.as_str()).unwrap(), Version::from(txn_data.version)).await;
                            package_registries.insert(account.to_string(), package_registry.unwrap().unwrap());
                        }
                        CallTraceWithSource::from(call_trace.clone().root().unwrap(), &package_registries)
                    },
                    _ => CallTraceWithSource::default(),
                }
            },
            _ => CallTraceWithSource::default(),
        };

        Ok(call_traces)
    }
}

pub struct SyncAptosTracer {
    debugger: Arc<dyn AptosTracerInterface + Send>,
}

impl SyncAptosTracer {
    pub fn new(debugger: Arc<dyn AptosTracerInterface + Send>) -> Self {
        Self { debugger }
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P) -> Result<Self> {
        Ok(Self::new(Arc::new(DBTracerInterface::open(
            db_root_path,
        )?)))
    }

    pub fn trace_transaction(
        &self,
        txn_hash: String,
    ) -> Result<CallTraceWithSource> {
        let txn_data = self.debugger.get_transaction_by_hash(txn_hash)?;
        let txn = txn_data.transaction;
        let state_view = SyncTracerView::new(self.debugger.clone(), Version::from(txn_data.version));
        let call_traces = match txn {
            Transaction::UserTransaction(user_txn) => {
                let txn_metadata = TransactionMetadata::new(&user_txn);
                match user_txn.payload() {
                    TransactionPayload::EntryFunction(entry_func) => {
                        let call_trace = AptosVM::get_call_trace(
                            &state_view,
                            entry_func.module().clone(),
                            entry_func.function().to_owned(),
                            entry_func.ty_args().to_vec(),
                            entry_func.args().to_vec(),
                            txn_metadata.senders(),
                            user_txn.max_gas_amount(),
                        ).unwrap();
                        // get all the package registries from accounts in call_trace
                        let mut package_registries = HashMap::new();
                        for account in &call_trace.1 {
                            let package_registry = self.debugger.get_package_registry(
                                AccountAddress::from_str(account.as_str()).unwrap(), Version::from(txn_data.version));
                            package_registries.insert(account.to_string(), package_registry.unwrap().unwrap());
                        }
                        CallTraceWithSource::from(call_trace.clone().root().unwrap(), &package_registries)
                    },
                    _ => CallTraceWithSource::default(),
                }
            },
            _ => CallTraceWithSource::default(),
        };

        Ok(call_traces)
    }
}

/// A call trace with source
///
/// This is a representation of the debug call trace
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CallTraceWithSource {
    pub module_id: String,
    pub func_name: String,
    pub inputs: Vec<String>,
    pub return_value: Vec<String>,
    pub type_args: Vec<String>,
    pub calls: Vec<CallTraceWithSource>,
    pub location: Location,
}

impl CallTraceWithSource {
    pub fn default() -> Self {
        CallTraceWithSource {
            module_id: "".to_string(),
            func_name: "".to_string(),
            inputs: vec![],
            return_value: vec![],
            type_args: vec![],
            calls: vec![],
            location: Location {
                account: "".to_string(),
                module: "".to_string(),
                lines: Range { start: Position { line: 0, column: 0 }, end: Position { line: 0, column: 0 } },
            },
        }
    }
    
    pub fn from(call_trace: InternalCallTrace, package_registries: &HashMap<String, PackageRegistry>) -> Self {
        let mut split_module = call_trace.module_id.split("::");
        let account = split_module.next();
        let module_name  = split_module.next();
        let mut files = Files::new();
        let mut call_trace_with_source = CallTraceWithSource {
            module_id: call_trace.module_id.to_string(),
            func_name: call_trace.func_name.to_string(),
            inputs: call_trace.inputs.clone(),
            return_value: call_trace.outputs.clone(),
            type_args: call_trace.type_args.clone(),
            calls: call_trace.sub_traces.clone().0.into_iter().map(|sub_trace| {
                CallTraceWithSource::from(sub_trace, package_registries)
            }).collect(),
            location: Location {
                account: account.unwrap().to_string(),
                module: module_name.unwrap().to_string(),
                lines: Range { start: Position { line: 0, column: 0 }, end: Position { line: 0, column: 0 } },
            },
        };
        package_registries.get(account.unwrap()).unwrap().packages.clone().into_iter().for_each(|package| {
            let matched_module = package.modules.into_iter().find(|module| {
                module.name.as_str() == module_name.unwrap()
            });
            match matched_module {
                None => {}
                Some(m) => {
                    if m.source_map.len() == 0 || m.source.len() == 0 {
                        return;
                    }
                    let source_map = unzip_metadata(&m.source_map).unwrap();
                    let source_code = unzip_metadata_str(&m.source).unwrap();
                    let file_id = files.add(module_name.unwrap(), source_code);
                    let deser_source_map: SourceMap = bcs::from_bytes(&source_map).unwrap();
                    let loc = deser_source_map.get_code_location(
                        FunctionDefinitionIndex::new(call_trace.fdef_idx as TableIndex),
                        CodeOffset::from(call_trace.pc)).unwrap();
                    let start_loc = files.location(file_id, loc.start()).unwrap();
                    let end_loc = files.location(file_id, loc.end()).unwrap();
                    call_trace_with_source.location.lines = Range {
                        start: Position { line: start_loc.line.0 as u32, column: start_loc.column.0 as u32 },
                        end: Position { line: end_loc.line.0 as u32, column: end_loc.column.0 as u32 }
                    };
                }
            }
        });
        call_trace_with_source
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Location {
    pub account: String,
    pub module: String,
    pub lines: Range,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Position {
    line: u32,
    column: u32,
}
