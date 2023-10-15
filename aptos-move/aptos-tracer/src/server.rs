// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{DebuggerServerConfig, SyncAptosTracer};
use anyhow::Result;
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path,
};
use std::future::Future;
use serde_json;

#[handler]
async fn call_trace(Path(hash): Path<(String)>) -> String {
    match std::env::var_os("DB_PATH") {
        Some(val) => {
            let db_path = val.to_str().unwrap();
            let tracer = SyncAptosTracer::db(db_path);
            let call_trace = tracer.unwrap().trace_transaction(hash).unwrap();
            serde_json::to_string_pretty(&call_trace).unwrap()
        }
        None => {
            "".to_string()
        }
    }
}

pub async fn run_debugger_server(
    config: DebuggerServerConfig,
) -> Result<()>  {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    std::env::set_var("DB_PATH", config.db_path);
    Server::new(TcpListener::bind((
        config.listen_address.clone(),
        config.listen_port,
    )))
        .run(Route::new().at("/call_trace/by_hash/:hash", get(call_trace)).with(cors)).await
        .map_err(anyhow::Error::msg)
}
