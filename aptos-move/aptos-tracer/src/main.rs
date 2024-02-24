// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_tracer::{DebuggerServerConfig, run_debugger_server};
use aptos_rest_client::Client;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use url::Url;
use std::collections::HashMap;

#[derive(Subcommand)]
pub enum Target {
    /// Use full node's rest api as query endpoint.
    Rest { endpoint: String, txn_hash: String, chain_id: u8 },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf, listen_address: Option<String>, listen_port: Option<u16> },
    /// Use a full node's rest api as query endpoint and run as a server.
    ServerBasedOnRest { endpoints: String, listen_address: Option<String>, listen_port: Option<u16> },
}
#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    target: Target,

    #[clap(long, default_value = "https://test.sentio.xyz")]
    sentio_endpoint: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new().init();
    let args = Argument::parse();
    match args.target {
        Target::Rest { endpoint, txn_hash, chain_id } => {
            // let tracer = AptosTracer::rest_client(Client::new(Url::parse(&endpoint)?), args.sentio_endpoint)?;
            // println!(
            //     "{}",
            //     serde_json::to_string_pretty(
            //         &tracer
            //         .trace_transaction(txn_hash, chain_id)
            //         .await?)?
            // );
            // Ok(())
            unimplemented!();
        },
        Target::DB { path,  listen_address, listen_port} => {
            // run as a server if the target is DB
            let mut config = DebuggerServerConfig::default();
            config.set_db_path(path);
            config.set_use_db(true);
            config.set_sentio_endpoint(args.sentio_endpoint);
            if let Some(address) = listen_address {
                config.listen_address = address;
            }
            if let Some(port) = listen_port {
                config.listen_port = port;
            }
            run_debugger_server(config).await
        },
        Target::ServerBasedOnRest { endpoints, listen_address, listen_port } => {
            // run as a server if the target is DB
            let mut config = DebuggerServerConfig::default();
            config.set_use_db(false);
            config.set_sentio_endpoint(args.sentio_endpoint);

            let mut endpoint_map: HashMap<u16, String> = HashMap::new(); 
            let chain_to_endpoint: Vec<&str> = endpoints.split(',').collect();
            for i in chain_to_endpoint.iter() {
                let t: Vec<&str> = i.split('=').collect();
                endpoint_map.insert(t[0].parse::<u16>().unwrap(), t[1].to_string());
            }
            config.set_rest_endpoints(endpoint_map);
            if let Some(address) = listen_address {
                config.listen_address = address;
            }
            if let Some(port) = listen_port {
                config.listen_port = port;
            }
            run_debugger_server(config).await
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
