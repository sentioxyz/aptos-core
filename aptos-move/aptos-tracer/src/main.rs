// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_tracer::AptosTracer;
use aptos_rest_client::Client;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use url::Url;

#[derive(Subcommand)]
pub enum Target {
    /// Use full node's rest api as query endpoint.
    Rest { endpoint: String },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf },
}
#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    target: Target,

    #[clap(long)]
    txn_hash: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new().init();
    let args = Argument::parse();

    let debugger = match args.target {
        Target::Rest { endpoint } => {
            AptosTracer::rest_client(Client::new(Url::parse(&endpoint)?))?
        },
        Target::DB { path } => AptosTracer::db(path)?,
    };

    println!(
        "{:#?}",
        debugger
            .trace_transaction(args.txn_hash)
            .await?
    );

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
