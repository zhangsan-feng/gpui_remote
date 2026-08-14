use std::sync::OnceLock;

use crate::application::agent_mcp::AgentMcpClient;

use super::{McpSettings, core};

static CONTROLLER: OnceLock<core::AgentMcpController> = OnceLock::new();

pub fn start(client: AgentMcpClient) {
    let controller = core::AgentMcpController::new(client);
    if CONTROLLER.set(controller).is_err() {
        log::warn!("Agent MCP service was already initialized");
        return;
    }

    if let Some(controller) = CONTROLLER.get() {
        let settings = core::load_settings();
        if let Err(error) = controller.apply(settings) {
            log::error!("启动 Agent MCP 服务失败: {error}");
        }
    }
}

pub fn settings() -> McpSettings {
    CONTROLLER
        .get()
        .map(core::AgentMcpController::settings)
        .unwrap_or_else(core::load_settings)
}

pub fn apply_settings(settings: McpSettings) -> Result<McpSettings, String> {
    let Some(controller) = CONTROLLER.get() else {
        return Err("MCP 服务尚未初始化".to_owned());
    };
    controller.apply(settings)
}
