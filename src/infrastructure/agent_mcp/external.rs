use std::sync::OnceLock;

use crate::{
    application::agent_mcp::{AgentMcpClient, AgentMcpDataFlow, AgentMcpQueryService},
    infrastructure::storage::SessionStorageRepository,
};

use super::{McpSettings, core, profile_query};

static CONTROLLER: OnceLock<core::AgentMcpController> = OnceLock::new();

pub fn start(client: AgentMcpClient, session: SessionStorageRepository) {
    let profile_query = profile_query::new(session);
    let queries = AgentMcpQueryService::new(profile_query);
    let data_flow = AgentMcpDataFlow::new(client, queries);
    let controller = core::AgentMcpController::new(data_flow);
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
