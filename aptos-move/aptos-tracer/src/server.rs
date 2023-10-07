// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosTracer, DebuggerServerConfig};
use anyhow::{anyhow, Result};
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path,
};
use std::future::Future;
use url::Url;
use aptos_rest_client::Client;
use serde_json;

#[handler]
async fn call_trace(Path((hash)): Path<(String)>) -> String {
    match std::env::var_os("APTOS_NODE_ENDPOINT") {
        Some(val) => {
            let endpoint = val.to_str().unwrap();
            let tracer = AptosTracer::rest_client(Client::new(Url::parse(endpoint).unwrap()));
            let call_trace = tracer.unwrap().trace_transaction(hash).await.unwrap();
            serde_json::to_string_pretty(&call_trace).unwrap()
        }
        None => {
            "".to_string()
        }
    }
}

pub fn run_debugger_server(
    config: DebuggerServerConfig,
) -> impl Future<Output = Result<(), std::io::Error>> {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    std::env::set_var("APTOS_NODE_ENDPOINT", config.node_endpoint);
    Server::new(TcpListener::bind((
        config.listen_address.clone(),
        config.listen_port,
    )))
        .run(Route::new().at("/call_trace/by_hash/:hash", get(call_trace)).with(cors))
}
