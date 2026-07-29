use std::{env, sync::Arc};

use anyhow::{Context as _, Result};
use axum::{Router, middleware};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::net::TcpListener;
use uuid::Uuid;

use crate::application::agent_mcp::AgentMcpClient;

use super::{auth::require_bearer_token, tools::AgentTerminalMcp};

const DEFAULT_PORT: u16 = 37_666;
const TOKEN_ENV: &str = "GPUI_REMOTE_MCP_TOKEN";
const PORT_ENV: &str = "GPUI_REMOTE_MCP_PORT";

pub(super) async fn run(client: AgentMcpClient) -> Result<()> {
    let port = env::var(PORT_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let token: Arc<str> = env::var(TOKEN_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
        .into();

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
    let address = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("绑定 MCP 服务地址失败: {address}"))?;

    log::info!("Agent MCP endpoint: http://{address}/mcp");
    log::info!("Agent MCP bearer token: {token}");
    axum::serve(listener, router)
        .await
        .context("运行 Agent MCP 服务失败")
}
