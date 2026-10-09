mod auth;
pub(crate) mod bridge;
mod core;
mod external;
mod server;
mod tools;

pub(crate) use external::AgentMcpRuntime;

pub(super) struct McpRuntime {
    service: AgentMcpRuntime,
    receiver: std::sync::Mutex<Option<bridge::McpBridgeReceiver>>,
}

impl McpRuntime {
    pub(super) fn new() -> Self {
        let (endpoint, receiver) = bridge::new();
        Self {
            service: AgentMcpRuntime::new(endpoint),
            receiver: std::sync::Mutex::new(Some(receiver)),
        }
    }
}

use serde::{Deserialize, Serialize};

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 37_666;
const SETTINGS_PATH: &str = "data/mcp.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSettings {
    pub enabled: bool,
    pub token_enabled: bool,
    pub host: String,
    pub port: u16,
    #[serde(skip)]
    pub token: String,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            token_enabled: false,
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            token: String::new(),
        }
    }
}
