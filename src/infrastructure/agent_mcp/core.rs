use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;
use uuid::Uuid;

use super::{McpSettings, SETTINGS_PATH, bridge::McpBridgeEndpoint, server};

const HOST_ENV: &str = "GPUI_REMOTE_MCP_HOST";
const PORT_ENV: &str = "GPUI_REMOTE_MCP_PORT";

#[derive(Clone)]
pub(super) struct AgentMcpController {
    state: Arc<Mutex<ControllerState>>,
    operation: Arc<Mutex<()>>,
}

struct ControllerState {
    bridge: McpBridgeEndpoint,
    settings: McpSettings,
    server_settings: Option<watch::Sender<McpSettings>>,
    server_task: Option<tokio::task::JoinHandle<()>>,
}

impl AgentMcpController {
    pub(super) fn new(bridge: McpBridgeEndpoint) -> Self {
        Self {
            state: Arc::new(Mutex::new(ControllerState {
                bridge,
                settings: McpSettings::default(),
                server_settings: None,
                server_task: None,
            })),
            operation: Arc::new(Mutex::new(())),
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
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "MCP 配置更新状态不可用".to_owned())?;
        self.apply_locked(&mut settings)
    }

    pub(super) fn initialize(&self) -> Result<McpSettings, String> {
        let _operation = self
            .operation
            .lock()
            .map_err(|_| "MCP 配置更新状态不可用".to_owned())?;
        let mut settings = load_settings();
        self.apply_locked(&mut settings)
    }

    pub(super) async fn shutdown(&self) -> Result<(), String> {
        let task = {
            let _operation = self
                .operation
                .lock()
                .map_err(|_| "MCP 配置更新状态不可用".to_owned())?;
            let mut state = self
                .state
                .lock()
                .map_err(|_| "MCP 服务状态不可用".to_owned())?;
            // Closing the channel stops the listener without persisting disabled settings.
            state.server_settings.take();
            state.server_task.take()
        };
        if let Some(task) = task {
            task.await
                .map_err(|error| format!("等待 MCP 服务关闭失败: {error}"))?;
        }
        Ok(())
    }

    fn apply_locked(&self, settings: &mut McpSettings) -> Result<McpSettings, String> {
        settings.token = Uuid::new_v4().to_string();
        save_settings(settings).map_err(|error| format!("保存 MCP 配置失败: {error}"))?;

        let mut state = self
            .state
            .lock()
            .map_err(|_| "MCP 服务状态不可用".to_owned())?;
        state.settings = settings.clone();
        if let Some(server_settings) = state.server_settings.as_ref() {
            server_settings
                .send(settings.clone())
                .map_err(|_| "MCP 服务重启任务不可用".to_owned())?;
        } else if state.settings.enabled {
            let (server_settings, receiver) = watch::channel(state.settings.clone());
            let bridge = state.bridge.clone();
            state.server_task = Some(tokio::spawn(run_server_manager(bridge, receiver)));
            state.server_settings = Some(server_settings);
        }

        Ok(settings.clone())
    }
}

async fn run_server_manager(bridge: McpBridgeEndpoint, mut settings: watch::Receiver<McpSettings>) {
    loop {
        let current_settings = settings.borrow().clone();
        if !current_settings.enabled {
            if settings.changed().await.is_err() {
                break;
            }
            continue;
        }

        let server_bridge = bridge.clone();
        let server_settings = current_settings.clone();
        let mut server =
            tokio::spawn(async move { server::run(server_bridge, server_settings).await });

        tokio::select! {
            changed = settings.changed() => {
                server.abort();
                let _ = server.await;
                if changed.is_err() {
                    break;
                }
            }
            result = &mut server => {
                match result {
                    Ok(Ok(())) => log::info!("Agent MCP server stopped"),
                    Ok(Err(error)) => log::error!("Agent MCP server stopped: {error:#}"),
                    Err(error) => log::error!("Agent MCP server task failed: {error}"),
                }
                if settings.changed().await.is_err() {
                    break;
                }
            }
        }
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
    settings
}

fn save_settings(settings: &McpSettings) -> std::io::Result<()> {
    if let Some(parent) = Path::new(SETTINGS_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(SETTINGS_PATH, bytes)
}
