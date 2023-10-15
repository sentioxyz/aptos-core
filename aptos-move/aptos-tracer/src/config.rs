// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
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
}

impl DebuggerServerConfig {
    pub fn default() -> Self {
        DebuggerServerConfig {
            disable: DebuggerServerConfig::default_disable(),
            listen_address: DebuggerServerConfig::default_listen_address(),
            listen_port: DebuggerServerConfig::default_listen_port(),
            db_path: DebuggerServerConfig::default_db_path()
        }
    }

    pub fn set_db_path(&mut self, db_path: PathBuf) {
        self.db_path = db_path
    }

    fn default_disable() -> bool {
        false
    }

    fn default_listen_address() -> String {
        "127.0.0.1".to_string()
    }

    fn default_listen_port() -> u16 {
        9102
    }

    fn default_db_path() -> PathBuf {PathBuf::new()}
}
