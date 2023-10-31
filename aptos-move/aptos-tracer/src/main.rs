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
    Rest { endpoint: String, txn_hash: String },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf },
}
#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    target: Target,
}

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new().init();
    let args = Argument::parse();

    match args.target {
        Target::Rest { endpoint, txn_hash } => {
            let tracer = AptosTracer::rest_client(Client::new(Url::parse(&endpoint)?))?;
            println!(
                "{:#?}",
                tracer
                    .trace_transaction(txn_hash)
                    .await?
            );
            Ok(())
        },
        Target::DB { path } => {
            // run as a server if the target is DB
            let mut config = DebuggerServerConfig::default();
            config.set_db_path(path);
            run_debugger_server(config).await
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
