use std::sync::OnceLock;

use crate::{
    data_context::{DataContext, GuiContextReceiver},
    infrastructure::data_context as infrastructure_context,
    infrastructure::storage::SessionStorageRepository,
};

use super::{McpSettings, core};

static CONTROLLER: OnceLock<core::AgentMcpController> = OnceLock::new();

pub fn start(session: SessionStorageRepository) -> GuiContextReceiver {
    let infrastructure = infrastructure_context::new(session);
    let (data_context, gui_receiver) = DataContext::new(infrastructure);
    let controller = core::AgentMcpController::new(data_context);
    if CONTROLLER.set(controller).is_err() {
        log::warn!("Agent MCP service was already initialized");
        return gui_receiver;
    }

    if let Some(controller) = CONTROLLER.get() {
        let settings = core::load_settings();
        if let Err(error) = controller.apply(settings) {
            log::error!("启动 Agent MCP 服务失败: {error}");
        }
    }

    gui_receiver
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
