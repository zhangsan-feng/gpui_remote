use super::{McpSettings, bridge::McpBridgeEndpoint, core};

pub(crate) struct AgentMcpRuntime {
    controller: core::AgentMcpController,
}

impl AgentMcpRuntime {
    pub(crate) fn new(bridge: McpBridgeEndpoint) -> Self {
        Self {
            controller: core::AgentMcpController::new(bridge),
        }
    }

    pub(crate) fn start(&self) {
        let settings = core::load_settings();
        if let Err(error) = self.controller.apply(settings) {
            log::error!("启动 Agent MCP 服务失败: {error}");
        }
    }

    pub(crate) fn settings(&self) -> McpSettings {
        self.controller.settings()
    }

    pub(crate) fn apply_settings(&self, settings: McpSettings) -> Result<McpSettings, String> {
        self.controller.apply(settings)
    }
}
