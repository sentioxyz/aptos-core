// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CallTraceWithSource, DebuggerServerConfig, SyncAptosTracer};
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path,
};
use anyhow::Result;
use poem::middleware::AddData;
use poem::web::{Data, Json};
use serde_json;
use url::Url;
use aptos_logger::info;
use aptos_rest_client::Client;

#[handler]
async fn call_trace(Path((chain_id, hash)): Path<(u8, String)>, config: Data<&DebuggerServerConfig>) -> Json<CallTraceWithSource> {
    let mut tracer;
    if config.use_db {
        tracer = SyncAptosTracer::db(config.db_path.to_str().unwrap(), config.clone().sentio_endpoint);
    } else {
        tracer = SyncAptosTracer::rest_client(
            Client::new(Url::parse(config.rest_endpoint.as_str()).unwrap()),
            config.clone().sentio_endpoint);
    }

    let call_trace = if let Some(hex) = hash.strip_prefix("0x") {
        tracer.unwrap().trace_transaction(hex.to_string(), chain_id).unwrap()
    } else {
        tracer.unwrap().trace_transaction(hash, chain_id).unwrap()
    };
    return Json(call_trace);
}

pub async fn run_debugger_server(
    config: DebuggerServerConfig,
) -> Result<()>  {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    // log the config
    info!("Debugger server config: {:?}", config);
    Server::new(TcpListener::bind((
        config.clone().listen_address,
        config.listen_port,
    )))
        .run(Route::new().at("/:chain_id/call_trace/by_hash/:hash", get(call_trace)).with(cors).with(AddData::new(config))).await
        .map_err(anyhow::Error::msg)
}
