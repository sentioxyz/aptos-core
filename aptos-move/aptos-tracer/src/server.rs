// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{CallTraceWithSource, DebuggerServerConfig, SyncAptosTracer};
use poem::{handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server, get, web::Path, IntoResponse, Response};
use anyhow::Result;
use poem::error::ResponseError;
use poem::http::StatusCode;
use poem::middleware::AddData;
use poem::web::{Data, Json};
use serde_json;
use url::Url;
use aptos_logger::{error, info};
use aptos_rest_client::Client;

#[handler]
async fn call_trace(Path((chain_id, hash)): Path<(u16, String)>, config: Data<&DebuggerServerConfig>) -> Result<Json<CallTraceWithSource>> {
    let mut tracer;
    if config.use_db {
        tracer = SyncAptosTracer::db(config.db_path.to_str().unwrap(), config.clone().sentio_endpoint);
    } else {
        let endpoint = match config.rest_endpoint_map.get(&chain_id) {
            Some(endpoint) => endpoint,
            None => &config.rest_endpoint
        };
        tracer = SyncAptosTracer::rest_client(
            Client::new(Url::parse(endpoint.as_str()).unwrap()),
            config.clone().sentio_endpoint);
    }

    let call_trace = if let Some(hex) = hash.strip_prefix("0x") {
        tracer.unwrap().trace_transaction(hex.to_string(), chain_id)
    } else {
        tracer.unwrap().trace_transaction(hash, chain_id)
    };
    match call_trace {
        Ok(trace_result) => {
            Ok(Json(trace_result))
        }
        Err(err) => {
            error!("Error: {:?}", err);
            Err(CustomError {
                message: format!("{:?}", err),
            }.into())
        }
    }
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
        .run(
            Route::new()
                .at("/:chain_id/call_trace/by_hash/:hash", get(call_trace))
                .with(cors)
                .with(AddData::new(config))).await
        .map_err(anyhow::Error::msg)
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct CustomError {
    message: String,
}

impl ResponseError for CustomError {
    fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
