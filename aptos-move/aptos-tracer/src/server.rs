// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CallTraceWithSource, DebuggerServerConfig, SyncAptosTracer};
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path,
};
use anyhow::Result;
use poem::web::Json;
use serde_json;

#[handler]
async fn call_trace(Path(hash): Path<String>) -> Json<CallTraceWithSource> {
    match std::env::var_os("DB_PATH") {
        Some(val) => {
            let db_path = val.to_str().unwrap();
            let tracer = SyncAptosTracer::db(db_path, std::env::var("SENTIO_ENDPOINT").unwrap());
            let call_trace = if let Some(hex) = hash.strip_prefix("0x") {
                tracer.unwrap().trace_transaction(hex.to_string()).unwrap()
            } else {
                tracer.unwrap().trace_transaction(hash).unwrap()
            };
            Json(call_trace)
        }
        None => {
            Json(CallTraceWithSource::default())
        }
    }
}

pub async fn run_debugger_server(
    config: DebuggerServerConfig,
    sentio_endpoint: String,
) -> Result<()>  {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    std::env::set_var("DB_PATH", config.db_path);
    std::env::set_var("SENTIO_ENDPOINT", sentio_endpoint);
    Server::new(TcpListener::bind((
        config.listen_address.clone(),
        config.listen_port,
    )))
        .run(Route::new().at("/call_trace/by_hash/:hash", get(call_trace)).with(cors)).await
        .map_err(anyhow::Error::msg)
}
