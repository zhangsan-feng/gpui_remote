use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Context as _, Result};
use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
};
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

    let stateless_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true);
    let stateless_bridge = bridge.clone();
    let stateless: StreamableHttpService<AgentTerminalMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(AgentTerminalMcp::new(stateless_bridge.clone())),
            Default::default(),
            stateless_config,
        );
    let stream_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(true)
        .with_json_response(true);
    let stream: StreamableHttpService<AgentTerminalMcp, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(AgentTerminalMcp::with_notifications(bridge.clone())),
            Default::default(),
            stream_config,
        );
    let mut router = Router::new()
        .route_service("/mcp", stateless)
        .route_service("/mcp/stream", stream)
        .layer(middleware::from_fn(trace_mcp_request));
    if settings.token_enabled {
        router = router.layer(middleware::from_fn_with_state(
            token.clone(),
            require_bearer_token,
        ));
    }
    router = router.layer(middleware::from_fn(only_get_post));
    let address = socket_address(&settings.host, settings.port);
    let listener = TcpListener::bind(&address)
        .await
        .with_context(|| format!("绑定 MCP 服务地址失败: {address}"))?;

    log::info!("Agent MCP endpoint: http://{address}/mcp");
    log::info!("Agent MCP notification endpoint: http://{address}/mcp/stream");
    if settings.token_enabled {
        log::info!("Agent MCP bearer authentication enabled");
    }
    axum::serve(listener, router)
        .await
        .context("运行 Agent MCP 服务失败")
}

async fn only_get_post(request: Request, next: middleware::Next) -> Response {
    if matches!(request.method(), &Method::GET | &Method::POST) {
        return next.run(request).await;
    }
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "GET, POST")],
        "Method Not Allowed",
    )
        .into_response()
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
