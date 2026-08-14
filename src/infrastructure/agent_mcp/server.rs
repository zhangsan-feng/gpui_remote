use std::sync::Arc;

use anyhow::{Context as _, Result};
use axum::{Router, middleware};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::net::TcpListener;

use crate::application::agent_mcp::AgentMcpClient;

use super::{McpSettings, auth::require_bearer_token, tools::AgentTerminalMcp};

pub(super) async fn run(client: AgentMcpClient, settings: McpSettings) -> Result<()> {
    let token: Arc<str> = settings.token.clone().into();

    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let service: StreamableHttpService<AgentTerminalMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(AgentTerminalMcp::new(client.clone())),
            Default::default(),
            config,
        );
    let router = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            token.clone(),
            require_bearer_token,
        ));
    let address = socket_address(&settings.host, settings.port);
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("绑定 MCP 服务地址失败: {address}"))?;

    log::info!("Agent MCP endpoint: http://{address}/mcp");
    log::info!("Agent MCP bearer token: {token}");
    axum::serve(listener, router)
        .await
        .context("运行 Agent MCP 服务失败")
}

fn socket_address(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
