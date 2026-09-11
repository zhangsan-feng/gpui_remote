use std::sync::{Arc, Mutex};

use gpui_kit::{App, Global};

use super::{
    agent_mcp::{self, AgentMcpRuntime, bridge::McpBridgeReceiver},
    profile_query,
    profile_query::QueryService,
    storage::{SessionStorageRepository, Storage},
};

struct McpRuntime {
    service: AgentMcpRuntime,
    receiver: Mutex<Option<McpBridgeReceiver>>,
}

struct InfrastructureContextInner {
    storage: Storage,
    profile_query: Arc<dyn profile_query::ProfileQuery>,
    mcp: McpRuntime,
}

#[derive(Clone)]
pub struct InfrastructureContext {
    inner: Arc<InfrastructureContextInner>,
}

impl Global for InfrastructureContext {}

impl InfrastructureContext {
    pub(crate) fn new() -> Self {
        let storage = Storage::new();
        let (endpoint, receiver) = agent_mcp::bridge::new();
        Self {
            inner: Arc::new(InfrastructureContextInner {
                profile_query: profile_query::sqlite(storage.session.clone()),
                storage,
                mcp: McpRuntime {
                    service: AgentMcpRuntime::new(endpoint),
                    receiver: Mutex::new(Some(receiver)),
                },
            }),
        }
    }

    pub(crate) fn start_mcp(&self, cx: &mut App) -> Result<(), String> {
        let receiver = self
            .inner
            .mcp
            .receiver
            .lock()
            .map_err(|_| "MCP bridge 状态不可用".to_owned())?
            .take()
            .ok_or_else(|| "MCP bridge 已经启动".to_owned())?;
        agent_mcp::bridge::start_mcp_bridge(cx, receiver);
        self.inner.mcp.service.start();
        log::info!("infrastructure_mcp_started");
        Ok(())
    }

    pub(crate) fn settings(&self) -> agent_mcp::McpSettings {
        self.inner.mcp.service.settings()
    }

    pub(crate) fn apply_settings(
        &self,
        settings: agent_mcp::McpSettings,
    ) -> Result<agent_mcp::McpSettings, String> {
        self.inner.mcp.service.apply_settings(settings)
    }

    pub(crate) fn session(&self) -> SessionStorageRepository {
        self.inner.storage.session.clone()
    }

    pub(crate) fn query_service(&self) -> QueryService {
        QueryService::new(Arc::clone(&self.inner.profile_query))
    }
}
