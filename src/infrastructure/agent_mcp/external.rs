use std::sync::OnceLock;

use crate::{
    data_context::DataContext, infrastructure::data_context as infrastructure_context,
    infrastructure::storage::SessionStorageRepository,
};

use super::{McpSettings, core};

static CONTROLLER: OnceLock<core::AgentMcpController> = OnceLock::new();
static DATA_CONTEXT: OnceLock<DataContext> = OnceLock::new();

pub fn start(session: SessionStorageRepository) -> DataContext {
    if let Some(data_context) = DATA_CONTEXT.get() {
        log::debug!("DataContext 已初始化，复用 ApplicationContext");
        return data_context.clone();
    }

    let infrastructure = infrastructure_context::new(session);
    let data_context = DataContext::new(infrastructure);
    if DATA_CONTEXT.set(data_context.clone()).is_err() {
        return DATA_CONTEXT
            .get()
            .expect("DataContext 已设置但无法读取")
            .clone();
    }
    let controller = core::AgentMcpController::new(data_context.clone());
    if CONTROLLER.set(controller).is_err() {
        log::warn!("Agent MCP service was already initialized");
        return data_context;
    }

    if let Some(controller) = CONTROLLER.get() {
        let settings = core::load_settings();
        if let Err(error) = controller.apply(settings) {
            log::error!("启动 Agent MCP 服务失败: {error}");
        }
    }

    data_context
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
