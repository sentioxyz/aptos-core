// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_tracer::{AptosTracer, DebuggerServerConfig, run_debugger_server};
use aptos_rest_client::Client;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use url::Url;

#[derive(Subcommand)]
pub enum Target {
    /// Use full node's rest api as query endpoint.
    Rest { endpoint: String, txn_hash: String, chain_id: u8 },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf, listen_address: Option<String>, listen_port: Option<u16> },
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
            let tracer = AptosTracer::rest_client(Client::new(Url::parse(&endpoint)?), args.sentio_endpoint)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &tracer
                    .trace_transaction(txn_hash, chain_id)
                    .await?)?
            );
            Ok(())
        },
        Target::DB { path,  listen_address, listen_port} => {
            // run as a server if the target is DB
            let mut config = DebuggerServerConfig::default();
            config.set_db_path(path);
            if let Some(address) = listen_address {
                config.listen_address = address;
            }
            if let Some(port) = listen_port {
                config.listen_port = port;
            }
            run_debugger_server(config, args.sentio_endpoint).await
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
