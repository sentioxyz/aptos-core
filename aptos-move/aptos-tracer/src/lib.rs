// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod config;
mod server;
mod sync_tracer_view;
mod sync_storage_interface;
mod converter;
mod sync_rest_interface;

pub use server::run_debugger_server;
pub use config::DebuggerServerConfig;

use anyhow::{format_err, Result};
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
use serde_json::Value;
use aptos_framework::natives::code::PackageRegistry;
use aptos_framework::{unzip_metadata, unzip_metadata_str};
use aptos_logger::{error, info};

use aptos_vm::transaction_metadata::TransactionMetadata;
use move_binary_format::file_format::{CodeOffset, FunctionDefinitionIndex, TableIndex};
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::account_address::AccountAddress;
use move_core_types::call_trace::{InternalCallTrace};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::ModuleId;
use crate::converter::move_value_to_json;
use crate::sync_rest_interface::RestTracerInterface;
use crate::sync_storage_interface::DBTracerInterface;
use crate::sync_tracer_view::{AptosTracerInterface, SyncTracerView};
use aptos_vm::data_cache::AsMoveResolver;

pub struct AptosTracer {
    debugger: Arc<dyn AptosValidatorInterface + Send>,
    sentio_endpoint: String,
}

impl AptosTracer {
    pub fn new(debugger: Arc<dyn AptosValidatorInterface + Send>, sentio_endpoint: String) -> Self {
        Self { debugger, sentio_endpoint }
    }

    pub fn rest_client(rest_client: Client, sentio_endpoint: String) -> Result<Self> {
        Ok(Self::new(Arc::new(RestDebuggerInterface::new(rest_client)), sentio_endpoint))
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P, sentio_endpoint: String) -> Result<Self> {
        Ok(Self::new(Arc::new(DBDebuggerInterface::open(
            db_root_path,
        )?), sentio_endpoint))
    }

    pub async fn trace_transaction(
        &self,
        txn_hash: String,
        chain_id: u8,
    ) -> Result<CallTraceWithSource> {
        let txn_data = self.debugger.get_transaction_by_hash(txn_hash).await?;
        let txn = txn_data.transaction;
        let state_view = DebuggerStateView::new(self.debugger.clone(), Version::from(txn_data.version - 1));
        let call_traces = match txn {
            Transaction::UserTransaction(user_txn) => {
                let txn_metadata = TransactionMetadata::new(&user_txn);
                let call_trace = AptosVM::get_call_trace(
                    &state_view,
                    user_txn.payload(),
                    txn_metadata.senders(),
                    user_txn.max_gas_amount(),
                ).map_err(|err| {
                    format_err!("Error getting call trace for txn_hash - {:?} : {:?}", txn_data.info.to_string(), err)
                })?;

                // get all the package names from accounts in call_trace
                let mut package_names = HashMap::new();
                for account in &call_trace.1 {
                    let package_registry = self.debugger.get_package_registry(
                        AccountAddress::from_str(account.as_str()).unwrap(), Version::from(txn_data.version)).await.unwrap();
                    match package_registry {
                        None => {}
                        Some(packages) => {
                            for package in packages.packages.into_iter() {
                                package_names.insert(package.name.clone(), account.to_string());
                            }
                        }
                    }
                }

                let module_id = match user_txn.payload() {
                    TransactionPayload::EntryFunction(entry_func) => Some(entry_func.module().clone()),
                    _ => None,
                };
                let mut modules_map = HashMap::new();
                if !module_id.is_none() {
                    let unwrapped_module_id = module_id.unwrap();
                    let package_registry = self.debugger.get_package_registry(
                        *unwrapped_module_id.clone().address(), Version::from(txn_data.version)).await.unwrap();
                    let matched_package = match package_registry {
                        None => {None}
                        Some(registry) => {
                            // find the module in the package registry
                            registry.packages.into_iter().find(|package| {
                                package.modules.clone().into_iter().find(|module| {
                                    module.name.as_str() == unwrapped_module_id.clone().name().as_str()
                                }).is_some()
                            })
                        }
                    };

                    match matched_package {
                        None => {}
                        Some(package) => {
                            let sentio_client = reqwest::Client::new();
                            let url = format!(
                                "{}/api/v1/move/fetch_and_compile?account={}&package={}&networkId={}&queryBytecode=true&querySource=true&querySourceMap=true",
                                self.sentio_endpoint,
                                unwrapped_module_id.clone().address(),
                                package.name,
                                chain_id);
                            info!("Fetching and compiling modules from {}", url);
                            let res = sentio_client.get(url).send().await;
                            match res {
                                Ok(resp_succeed) => {
                                    let compile_response: CompileResponse = resp_succeed.json().await.unwrap_or(CompileResponse {
                                        result: PackageCompilation {
                                            name: "".to_string(),
                                            moduleWithoutCode: None,
                                            modules: vec![],
                                            dependencies: None,
                                        }
                                    });
                                    compile_response.result.modules.into_iter().for_each(|module| {
                                        modules_map.insert(ModuleId::new(
                                            unwrapped_module_id.clone().address().clone(),
                                            Identifier::new(module.name.as_str()).unwrap()).to_string(), module);
                                    });
                                    match compile_response.result.dependencies {
                                        None => {}
                                        Some(dependencies) => {
                                            dependencies.into_iter().for_each(|dependency| {
                                                dependency.modules.into_iter().for_each(|module| {
                                                    let account_address = package_names.get(dependency.name.as_str());
                                                    match account_address {
                                                        None => {}
                                                        Some(account) => {
                                                            modules_map.insert(
                                                                ModuleId::new(
                                                                    AccountAddress::from_str(account.as_str()).unwrap(),
                                                                    Identifier::new(module.name.as_str()).unwrap()).to_string(),
                                                                module);
                                                        }
                                                    }
                                                });
                                            });
                                        }
                                    }
                                }
                                Err(error) => {
                                    error!("Error fetching and compiling modules: {:?}", error);
                                }
                            }
                        }
                    }
                }

                CallTraceWithSource::from_modules(call_trace.clone().root().unwrap(), &modules_map)
            },
            _ => CallTraceWithSource::default(),
        };

        Ok(call_traces)
    }
}

pub struct SyncAptosTracer {
    debugger: Arc<dyn AptosTracerInterface + Send>,
    sentio_endpoint: String,
}

impl SyncAptosTracer {
    pub fn new(debugger: Arc<dyn AptosTracerInterface + Send>, sentio_endpoint: String) -> Self {
        Self { debugger, sentio_endpoint }
    }

    pub fn rest_client(rest_client: Client, sentio_endpoint: String) -> Result<Self> {
        Ok(Self::new(Arc::new(RestTracerInterface::new(rest_client)), sentio_endpoint))
    }

    pub fn db<P: AsRef<Path> + Clone>(db_root_path: P, sentio_endpoint: String) -> Result<Self> {
        Ok(Self::new(Arc::new(DBTracerInterface::open(
            db_root_path,
        )?), sentio_endpoint))
    }

    pub fn trace_transaction(
        &self,
        txn_hash: String,
        chain_id: u8,
    ) -> Result<CallTraceWithSource> {
        let txn_data = self.debugger.get_transaction_by_hash(txn_hash)?;
        let txn = txn_data.transaction;
        let state_view = SyncTracerView::new(self.debugger.clone(), Version::from(txn_data.version - 1));
        let call_traces = match txn {
            Transaction::UserTransaction(user_txn) => {
                let txn_metadata = TransactionMetadata::new(&user_txn);
                let call_trace = AptosVM::get_call_trace(
                    &state_view,
                    user_txn.payload(),
                    txn_metadata.senders(),
                    user_txn.max_gas_amount(),
                ).map_err(|err| {
                    format_err!("Error getting call trace for txn_hash - {:?} : {:?}", txn_data.info.to_string(), err)
                })?;

                // get all the package names from accounts in call_trace
                let mut package_names = HashMap::new();
                for account in &call_trace.1 {
                    let package_registry = self.debugger.get_package_registry(
                        AccountAddress::from_str(account.as_str()).unwrap(), Version::from(txn_data.version)).unwrap();
                    match package_registry {
                        None => {}
                        Some(packages) => {
                            for package in packages.packages.into_iter() {
                                package_names.insert(package.name.clone(), account.to_string());
                            }
                        }
                    }
                }

                let module_id = match user_txn.payload() {
                    TransactionPayload::EntryFunction(entry_func) => Some(entry_func.module().clone()),
                    _ => None,
                };
                let mut modules_map = HashMap::new();
                if !module_id.is_none() {
                    let unwrapped_module_id = module_id.unwrap();
                    let package_registry = self.debugger.get_package_registry(
                        *unwrapped_module_id.clone().address(), Version::from(txn_data.version)).unwrap();
                    let matched_package = match package_registry {
                        None => {None}
                        Some(registry) => {
                            // find the module in the package registry
                            registry.packages.into_iter().find(|package| {
                                package.modules.clone().into_iter().find(|module| {
                                    module.name.as_str() == unwrapped_module_id.clone().name().as_str()
                                }).is_some()
                            })
                        }
                    };

                    match matched_package {
                        None => {}
                        Some(package) => {
                            let url = format!(
                                "{}/api/v1/move/fetch_and_compile?account={}&package={}&networkId={}&queryBytecode=true&querySource=true&querySourceMap=true",
                                self.sentio_endpoint,
                                unwrapped_module_id.clone().address(),
                                package.name,
                                chain_id);
                            info!("Fetching and compiling modules from {}", url);
                            let compile_response = std::thread::spawn(move || {
                                match reqwest::blocking::get(url) {
                                    Ok(resp_succeed) => {
                                        let compile_response: CompileResponse = resp_succeed.json().unwrap_or(CompileResponse {
                                            result: PackageCompilation {
                                                name: "".to_string(),
                                                moduleWithoutCode: None,
                                                modules: vec![],
                                                dependencies: None,
                                            }
                                        });
                                        compile_response
                                    }
                                    Err(error) => {
                                        error!("Error fetching and compiling modules: {:?}", error);
                                        CompileResponse {
                                            result: PackageCompilation {
                                                name: "".to_string(),
                                                moduleWithoutCode: None,
                                                modules: vec![],
                                                dependencies: None,
                                            }
                                        }
                                    }
                                }
                            }).join().unwrap();

                            compile_response.result.modules.into_iter().for_each(|module| {
                                modules_map.insert(ModuleId::new(
                                    unwrapped_module_id.clone().address().clone(),
                                    Identifier::new(module.name.as_str()).unwrap()).to_string(), module);
                            });
                            match compile_response.result.dependencies {
                                None => {}
                                Some(dependencies) => {
                                    dependencies.into_iter().for_each(|dependency| {
                                        dependency.modules.into_iter().for_each(|module| {
                                            let account_address = package_names.get(dependency.name.as_str());
                                            match account_address {
                                                None => {}
                                                Some(account) => {
                                                    modules_map.insert(
                                                        ModuleId::new(
                                                            AccountAddress::from_str(account.as_str()).unwrap(),
                                                            Identifier::new(module.name.as_str()).unwrap()).to_string(),
                                                        module);
                                                }
                                            }
                                        });
                                    });
                                }
                            }
                        }
                    }
                }

                CallTraceWithSource::from_modules(call_trace.clone().root().unwrap(), &modules_map)
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
#[serde(rename_all = "camelCase")]
pub struct CallTraceWithSource {
    pub from: String,
    pub to: String,
    pub contract_name: String,
    pub function_name: String,
    pub inputs: Vec<Value>,
    pub return_value: Vec<Value>,
    pub type_args: Vec<String>,
    pub calls: Vec<CallTraceWithSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
}

impl CallTraceWithSource {
    pub fn default() -> Self {
        CallTraceWithSource {
            from: "".to_string(),
            to: "".to_string(),
            contract_name: "".to_string(),
            function_name: "".to_string(),
            inputs: vec![],
            return_value: vec![],
            type_args: vec![],
            calls: vec![],
            location: None,
        }
    }
    
    pub fn from(call_trace: InternalCallTrace, package_registries: &HashMap<String, PackageRegistry>) -> Self {
        let mut split_module = call_trace.from_module_id.split("::");
        let account = split_module.next();
        let module_name  = split_module.next();
        let mut split_to_module = call_trace.module_id.split("::");
        let to_account = split_to_module.next();
        let to_module_name  = split_to_module.next();
        let mut files = Files::new();
        let mut call_trace_with_source = CallTraceWithSource {
            from: account.unwrap().to_string(),
            contract_name: module_name.unwrap().to_string(),
            to: to_account.unwrap().to_string(),
            function_name: format!("{}::{}", to_module_name.unwrap().to_string(), call_trace.func_name),
            inputs: call_trace.inputs.clone().into_iter().map(|i| move_value_to_json(i)).collect::<Vec<Value>>(),
            return_value: call_trace.outputs.clone().into_iter().map(|i| move_value_to_json(i)).collect::<Vec<Value>>(),
            type_args: call_trace.type_args.clone(),
            calls: call_trace.sub_traces.clone().0.into_iter().map(|sub_trace| {
                CallTraceWithSource::from(sub_trace, package_registries)
            }).collect(),
            location: None,
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
                    call_trace_with_source.location = Some(Location {
                        account: account.unwrap().to_string(),
                        module: module_name.unwrap().to_string(),
                        lines: Range {
                            start: Position { line: start_loc.line.0 as u32, column: start_loc.column.0 as u32 },
                            end: Position { line: end_loc.line.0 as u32, column: end_loc.column.0 as u32 }
                    }});
                }
            }
        });
        call_trace_with_source
    }

    pub fn from_modules(call_trace: InternalCallTrace, modules_map: &HashMap<String, ModuleCompilation>) -> Self {
        let mut split_module = call_trace.from_module_id.split("::");
        let account = split_module.next();
        let module_name  = split_module.next();
        let mut split_to_module = call_trace.module_id.split("::");
        let to_account = split_to_module.next();
        let to_module_name  = split_to_module.next();
        let mut files = Files::new();
        let mut call_trace_with_source = CallTraceWithSource {
            from: account.unwrap().to_string(),
            contract_name: module_name.unwrap().to_string(),
            to: to_account.unwrap().to_string(),
            function_name: format!("{}::{}", to_module_name.unwrap().to_string(), call_trace.func_name),
            inputs: call_trace.inputs.clone().into_iter().map(|i| move_value_to_json(i)).collect::<Vec<Value>>(),
            return_value: call_trace.outputs.clone().into_iter().map(|i| move_value_to_json(i)).collect::<Vec<Value>>(),
            type_args: call_trace.type_args.clone(),
            calls: call_trace.sub_traces.clone().0.into_iter().map(|sub_trace| {
                CallTraceWithSource::from_modules(sub_trace, modules_map)
            }).collect(),
            location: None,
        };
        let module = modules_map.get(call_trace.from_module_id.as_str());
        match module {
            None => {}
            Some(module_info) => {
                if module_info.sourceMap.len() == 0 || module_info.source.len() == 0 {
                    return call_trace_with_source;
                }
                let source_map = hex::decode(module_info.sourceMap.clone().strip_prefix("0x").unwrap()).unwrap();
                let file_id = files.add(module_name.unwrap(), module_info.source.clone());
                let deser_source_map = bcs::from_bytes::<SourceMap>(&source_map)
                    .map_err(|_| format_err!("Error deserializing into source map"));
                match deser_source_map {
                    Ok(valid_source_map) => {
                        let loc = valid_source_map.get_code_location(
                            FunctionDefinitionIndex::new(call_trace.fdef_idx as TableIndex),
                            CodeOffset::from(call_trace.pc));
                        match loc {
                            Ok(valid_loc) => {
                                let start_loc = files.location(file_id, valid_loc.start()).unwrap_or_else(|_| {
                                    error!("Error getting code location for call trace - {:?} : {:?}", call_trace, "start_loc is None");
                                    return files.location(file_id, 0).unwrap();
                                });
                                let end_loc = files.location(file_id, valid_loc.end()).unwrap_or_else(|_| {
                                    error!("Error getting code location for call trace - {:?} : {:?}", call_trace, "end_loc is None");
                                    return files.location(file_id, 0).unwrap();
                                });
                                call_trace_with_source.location = Some(Location {
                                    account: account.unwrap().to_string(),
                                    module: module_name.unwrap().to_string(),
                                    lines: Range {
                                        start: Position { line: start_loc.line.0 as u32, column: start_loc.column.0 as u32 },
                                        end: Position { line: end_loc.line.0 as u32, column: end_loc.column.0 as u32 }
                                }});
                            }
                            Err(err) => {
                                error!("Error getting code location for module - {:?}, function index - {:?}, pc - {:?} : {:?}",
                                    call_trace.from_module_id,
                                    call_trace.fdef_idx,
                                    call_trace.pc,
                                    err);
                                return call_trace_with_source;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error deserializing into source map: {:?}", err);
                        return call_trace_with_source;
                    }
                }
            }
        }
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

#[derive(Clone, Deserialize, Serialize)]
struct CompileResponse {
    result: PackageCompilation,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PackageCompilation {
    name: String,
    moduleWithoutCode: Option<Vec<String>>,
    modules: Vec<ModuleCompilation>,
    dependencies: Option<Vec<PackageCompilation>>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ModuleCompilation {
    name: String,
    sourceMap: String,
    source: String,
    bytecode: String,
    abi: Option<HashMap<String, String>>,
}
