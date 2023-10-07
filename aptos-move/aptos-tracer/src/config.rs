// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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

    /// Aptos node rest endpoint
    #[serde(default = "DebuggerServerConfig::default_node_endpoint")]
    pub node_endpoint: String,
}

impl DebuggerServerConfig {
    fn default_disable() -> bool {
        false
    }

    fn default_listen_address() -> String {
        "0.0.0.0".to_string()
    }

    fn default_listen_port() -> u16 {
        9102
    }

    fn default_node_endpoint() -> String {
        "https://fullnode.mainnet.aptoslabs.com/v1".to_string()
    }
}
