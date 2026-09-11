use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Context as _, Result};
use axum::{Router, extract::Request, middleware, response::Response};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::net::TcpListener;

static MCP_HTTP_REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

use super::{
    McpSettings, auth::require_bearer_token, bridge::McpBridgeEndpoint, tools::AgentTerminalMcp,
};

pub(super) async fn run(bridge: McpBridgeEndpoint, settings: McpSettings) -> Result<()> {
    let token: Arc<str> = settings.token.clone().into();

    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let service: StreamableHttpService<AgentTerminalMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(AgentTerminalMcp::new(bridge.clone())),
            Default::default(),
            config,
        );
    let router = Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn(trace_mcp_request))
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

async fn trace_mcp_request(request: Request, next: middleware::Next) -> Response {
    let trace_id = MCP_HTTP_REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let started = std::time::Instant::now();
    log::debug!("MCP HTTP request started: trace_id={trace_id}, method={method}, path={path}");
    let response = next.run(request).await;
    log::debug!(
        "MCP HTTP request finished: trace_id={trace_id}, method={method}, path={path}, status={}, elapsed_ms={}",
        response.status(),
        started.elapsed().as_millis()
    );
    response
}

fn socket_address(host: &str, port: u16) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
