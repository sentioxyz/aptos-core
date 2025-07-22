// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, path::PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DebuggerServerConfig {
    /// Whether to disable the debugger server.
    #[serde(default = "DebuggerServerConfig::default_disable")]
    pub disable: bool,

    /// What address to listen on, e.g. localhost / 0.0.0.0
    #[serde(default = "DebuggerServerConfig::default_listen_address")]
    pub listen_address: String,

    /// What port to listen on.
    #[serde(default = "DebuggerServerConfig::default_listen_port")]
    pub listen_port: u16,

    /// DB path
    #[serde(default = "DebuggerServerConfig::default_db_path")]
    pub db_path: PathBuf,

    #[serde(default = "DebuggerServerConfig::default_rest_endpoint_map")]
    pub rest_endpoint_map: HashMap<u16, String>,

    /// use db or not
    #[serde(default = "DebuggerServerConfig::default_use_db")]
    pub use_db: bool,

    /// sentio endpoint
    #[serde(default = "DebuggerServerConfig::default_sentio_endpoint")]
    pub sentio_endpoint: String,
}

impl DebuggerServerConfig {
    pub fn default() -> Self {
        DebuggerServerConfig {
            disable: DebuggerServerConfig::default_disable(),
            listen_address: DebuggerServerConfig::default_listen_address(),
            listen_port: DebuggerServerConfig::default_listen_port(),
            db_path: DebuggerServerConfig::default_db_path(),
            rest_endpoint_map: DebuggerServerConfig::default_rest_endpoint_map(),
            use_db: DebuggerServerConfig::default_use_db(),
            sentio_endpoint: DebuggerServerConfig::default_sentio_endpoint(),
        }
    }

    pub fn set_db_path(&mut self, db_path: PathBuf) {
        self.db_path = db_path
    }

    pub fn set_rest_endpoints(&mut self, rest_endpoint_map: HashMap<u16, String>) {
        self.rest_endpoint_map = rest_endpoint_map
    }

    pub fn set_use_db(&mut self, use_db: bool) {
        self.use_db = use_db
    }

    pub fn set_sentio_endpoint(&mut self, sentio_endpoint: String) {
        self.sentio_endpoint = sentio_endpoint
    }

    fn default_disable() -> bool {
        false
    }

    fn default_listen_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_listen_port() -> u16 {
        9201
    }

    fn default_db_path() -> PathBuf {PathBuf::new()}

    fn default_rest_endpoint() -> String {
        "https://fullnode.mainnet.aptoslabs.com/v1".to_string()
    }

    fn default_rest_endpoint_map() -> HashMap<u16, String> {
        HashMap::from([
            (1, "https://fullnode.mainnet.aptoslabs.com/v1".to_string()),
            (2001, "https://aptos.testnet.suzuka.movementlabs.xyz/v1".to_string())
        ])
    }

    fn default_use_db() -> bool {
        false
    }

    fn default_sentio_endpoint() -> String {
        "https://test.sentio.xyz".to_string()
    }
}
