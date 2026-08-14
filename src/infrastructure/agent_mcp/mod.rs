mod auth;
mod core;
mod external;
mod server;
mod tools;

pub use external::{apply_settings, settings, start};

use serde::{Deserialize, Serialize};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 37_666;
const SETTINGS_PATH: &str = "data/mcp.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct McpSettings {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    #[serde(skip)]
    pub token: String,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            token: String::new(),
        }
    }
}
