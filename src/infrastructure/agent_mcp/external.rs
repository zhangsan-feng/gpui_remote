use super::{McpSettings, bridge::McpBridgeEndpoint, core};
use crate::{
    application::{Application, ports::McpSettingsFuture},
    infrastructure::InfrastructureContext,
};

pub(crate) struct AgentMcpRuntime {
    controller: core::AgentMcpController,
}

impl AgentMcpRuntime {
    pub(crate) fn new(bridge: McpBridgeEndpoint) -> Self {
        Self {
            controller: core::AgentMcpController::new(bridge),
        }
    }

    pub(crate) async fn start(&self) -> Result<(), String> {
        let controller = self.controller.clone();
        let _settings = tokio::task::spawn_blocking(move || controller.initialize())
            .await
            .map_err(|error| format!("启动 Agent MCP 服务任务失败: {error}"))??;
        Ok(())
    }

    pub(crate) fn settings(&self) -> McpSettings {
        self.controller.settings()
    }

    pub(crate) async fn shutdown(&self) -> Result<(), String> {
        self.controller.shutdown().await
    }

    pub(crate) fn apply_settings(&self, settings: McpSettings) -> Result<McpSettings, String> {
        self.controller.apply(settings)
    }
}

impl InfrastructureContext {
    pub(crate) async fn start_mcp(&self, application: Application) -> Result<(), String> {
        let receiver = self
            .inner
            .mcp
            .receiver
            .lock()
            .map_err(|_| "MCP bridge 状态不可用".to_owned())?
            .take()
            .ok_or_else(|| "MCP bridge 已经启动".to_owned())?;
        super::bridge::start_mcp_bridge(application, receiver)?;
        self.inner.mcp.service.start().await?;
        log::info!("infrastructure_mcp_started");
        Ok(())
    }

    pub(crate) async fn shutdown_mcp(&self) -> Result<(), String> {
        self.inner.mcp.service.shutdown().await?;
        log::info!("infrastructure_mcp_stopped");
        Ok(())
    }

    pub(crate) fn current_mcp_settings(&self) -> McpSettings {
        self.inner.mcp.service.settings()
    }

    pub(crate) fn update_mcp_settings(&self, settings: McpSettings) -> Result<McpSettings, String> {
        self.inner.mcp.service.apply_settings(settings)
    }

    pub(crate) fn mcp_settings(&self) -> crate::application::mcp::McpSettings {
        let settings = self.current_mcp_settings();
        crate::application::mcp::McpSettings {
            enabled: settings.enabled,
            token_enabled: settings.token_enabled,
            host: settings.host,
            port: settings.port,
            token: settings.token,
        }
    }

    pub(crate) fn save_mcp_settings(
        &self,
        settings: crate::application::mcp::McpSettings,
    ) -> McpSettingsFuture {
        let infrastructure = self.clone();
        Box::pin(async move {
            let settings = McpSettings {
                enabled: settings.enabled,
                token_enabled: settings.token_enabled,
                host: settings.host,
                port: settings.port,
                token: settings.token,
            };
            let settings =
                tokio::task::spawn_blocking(move || infrastructure.update_mcp_settings(settings))
                    .await
                    .map_err(|error| format!("更新 MCP 配置任务失败: {error}"))??;
            Ok(crate::application::mcp::McpSettings {
                enabled: settings.enabled,
                token_enabled: settings.token_enabled,
                host: settings.host,
                port: settings.port,
                token: settings.token,
            })
        })
    }
}
