use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use tokio::task::AbortHandle;
use uuid::Uuid;

use crate::application::agent_mcp::AgentMcpClient;

use super::{McpSettings, SETTINGS_PATH, server};

const HOST_ENV: &str = "GPUI_REMOTE_MCP_HOST";
const PORT_ENV: &str = "GPUI_REMOTE_MCP_PORT";

pub(super) struct AgentMcpController {
    state: Arc<Mutex<ControllerState>>,
}

struct ControllerState {
    client: AgentMcpClient,
    settings: McpSettings,
    server_abort: Option<AbortHandle>,
}

impl AgentMcpController {
    pub(super) fn new(client: AgentMcpClient) -> Self {
        Self {
            state: Arc::new(Mutex::new(ControllerState {
                client,
                settings: McpSettings::default(),
                server_abort: None,
            })),
        }
    }

    pub(super) fn settings(&self) -> McpSettings {
        self.state
            .lock()
            .expect("MCP controller lock poisoned")
            .settings
            .clone()
    }

    pub(super) fn apply(&self, mut settings: McpSettings) -> Result<McpSettings, String> {
        settings.token = Uuid::new_v4().to_string();
        save_settings(&settings).map_err(|error| format!("保存 MCP 配置失败: {error}"))?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| "MCP 服务状态不可用".to_owned())?;
        if let Some(server_abort) = state.server_abort.take() {
            server_abort.abort();
        }

        state.settings = settings.clone();
        if settings.enabled {
            let client = state.client.clone();
            let server_settings = settings.clone();
            let server = tokio::spawn(async move {
                if let Err(error) = server::run(client, server_settings).await {
                    log::error!("Agent MCP server stopped: {error:#}");
                }
            });
            state.server_abort = Some(server.abort_handle());
        }

        Ok(settings)
    }
}

pub(super) fn load_settings() -> McpSettings {
    let file_exists = Path::new(SETTINGS_PATH).exists();
    let mut settings = fs::read(SETTINGS_PATH)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<McpSettings>(&bytes).ok())
        .unwrap_or_default();

    if !file_exists {
        if let Ok(host) = std::env::var(HOST_ENV) {
            if !host.trim().is_empty() {
                settings.host = host;
            }
        }
        if let Some(port) = std::env::var(PORT_ENV)
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
        {
            if port != 0 {
                settings.port = port;
            }
        }
    }

    if settings.host.trim().is_empty() {
        settings.host = super::DEFAULT_HOST.to_owned();
    }
    if settings.port == 0 {
        settings.port = super::DEFAULT_PORT;
    }
    settings.token = Uuid::new_v4().to_string();
    if let Err(error) = save_settings(&settings) {
        log::error!("保存 MCP 配置失败: {error}");
    }

    settings
}

fn save_settings(settings: &McpSettings) -> std::io::Result<()> {
    if let Some(parent) = Path::new(SETTINGS_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(SETTINGS_PATH, bytes)
}
