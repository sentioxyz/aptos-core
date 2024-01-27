// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CallTraceWithSource, DebuggerServerConfig, SyncAptosTracer};
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path,
};
use anyhow::Result;
use poem::web::Json;
use serde_json;
use url::Url;
use aptos_rest_client::Client;

#[handler]
async fn call_trace(Path((chain_id, hash)): Path<(u8, String)>) -> Json<CallTraceWithSource> {
    // check db or rest endpoint
    let db_path = std::env::var_os("DB_PATH");
    let rest_endpoint = std::env::var_os("REST_ENDPOINT");
    if db_path.is_none() && rest_endpoint.is_none() {
        println!("Please set DB_PATH or REST_ENDPOINT");
        return Json(CallTraceWithSource::default());
    }
    let mut tracer;
    if rest_endpoint.is_some() {
        tracer = SyncAptosTracer::rest_client(
            Client::new(Url::parse(&rest_endpoint.unwrap().to_str().unwrap()).unwrap()),
            std::env::var("SENTIO_ENDPOINT").unwrap());
    } else {
        tracer = SyncAptosTracer::db(db_path.unwrap().to_str().unwrap(), std::env::var("SENTIO_ENDPOINT").unwrap());
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
    sentio_endpoint: String,
    with_db: bool,
) -> Result<()>  {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    if with_db {
        std::env::set_var("DB_PATH", config.db_path);
    } else {
        std::env::set_var("REST_ENDPOINT", config.rest_endpoint);
    }
    std::env::set_var("SENTIO_ENDPOINT", sentio_endpoint);
    Server::new(TcpListener::bind((
        config.listen_address.clone(),
        config.listen_port,
    )))
        .run(Route::new().at("/:chain_id/call_trace/by_hash/:hash", get(call_trace)).with(cors)).await
        .map_err(anyhow::Error::msg)
}
