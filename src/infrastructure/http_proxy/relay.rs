use std::{
    convert::Infallible,
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use http_body_util::{BodyExt, Full, combinators::UnsyncBoxBody};
use hyper::{
    HeaderMap, Method, Request, Response, StatusCode, Uri,
    body::{Bytes, Incoming},
    header::{CONNECTION, HOST, HeaderName, HeaderValue, PROXY_AUTHENTICATE, PROXY_AUTHORIZATION},
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::{net::TcpStream, sync::mpsc, task::JoinSet, time::timeout};

use crate::domain::http_proxy::HttpProxyConnection;

use super::ConnectionEvent;

type ProxyBody = UnsyncBoxBody<Bytes, hyper::Error>;
type RelayTask = Pin<Box<dyn Future<Output = ()> + Send>>;

enum ClientJob {
    Upstream(RelayTask),
    Tunnel(RelayTask),
}

struct ConnectionStatusGuard {
    id: u64,
    events: mpsc::UnboundedSender<ConnectionEvent>,
}

impl ConnectionStatusGuard {
    fn new(
        events: mpsc::UnboundedSender<ConnectionEvent>,
        connection: HttpProxyConnection,
    ) -> Self {
        let id = connection.id;
        let _ = events.send(ConnectionEvent::Opened(connection));
        Self { id, events }
    }
}

impl Drop for ConnectionStatusGuard {
    fn drop(&mut self) {
        let _ = self.events.send(ConnectionEvent::Closed(self.id));
    }
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HEADER_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn handle_client(
    client: TcpStream,
    client_address: SocketAddr,
    proxy_address: SocketAddr,
    credentials: Option<(String, String)>,
    connection_events: mpsc::UnboundedSender<ConnectionEvent>,
    connection_ids: Arc<AtomicU64>,
) -> io::Result<()> {
    let credentials = Arc::new(credentials);
    let (jobs, mut receiver) = mpsc::unbounded_channel::<ClientJob>();
    let service = service_fn(move |request| {
        let credentials = credentials.clone();
        let jobs = jobs.clone();
        let connection_events = connection_events.clone();
        let connection_ids = connection_ids.clone();
        async move {
            Ok::<_, Infallible>(
                handle_request(
                    request,
                    credentials.as_ref().as_ref(),
                    &jobs,
                    client_address,
                    proxy_address,
                    &connection_events,
                    &connection_ids,
                )
                .await,
            )
        }
    });
    let mut builder = hyper::server::conn::http1::Builder::new();
    builder
        .timer(TokioTimer::new())
        .header_read_timeout(HEADER_TIMEOUT)
        .max_buf_size(32 * 1024);
    let connection = builder
        .serve_connection(TokioIo::new(client), service)
        .with_upgrades();
    tokio::pin!(connection);
    // The JoinSet owns both upstream drivers and upgraded tunnels. Dropping this
    // client future during shutdown aborts every child and releases its sockets.
    let mut upstreams = JoinSet::new();
    let mut tunnels = JoinSet::new();
    loop {
        tokio::select! {
            result = &mut connection => {
                result.map_err(io::Error::other)?;
                break;
            }
            Some(job) = receiver.recv() => { spawn_job(job, &mut upstreams, &mut tunnels); }
            _ = upstreams.join_next(), if !upstreams.is_empty() => {}
            _ = tunnels.join_next(), if !tunnels.is_empty() => {}
        }
    }
    // An upgrade completes the HTTP connection before its tunnel completes.
    // Drain jobs queued by the final request before waiting for its children.
    while let Ok(job) = receiver.try_recv() {
        spawn_job(job, &mut upstreams, &mut tunnels);
    }
    // The downstream has consumed its HTTP body (or disconnected), so any
    // remaining upstream driver can close immediately. Only tunnels outlive it.
    upstreams.shutdown().await;
    while tunnels.join_next().await.is_some() {}
    Ok(())
}

fn spawn_job(job: ClientJob, upstreams: &mut JoinSet<()>, tunnels: &mut JoinSet<()>) {
    match job {
        ClientJob::Upstream(task) => {
            upstreams.spawn(task);
        }
        ClientJob::Tunnel(task) => {
            tunnels.spawn(task);
        }
    }
}

async fn handle_request(
    mut request: Request<Incoming>,
    credentials: Option<&(String, String)>,
    jobs: &mpsc::UnboundedSender<ClientJob>,
    client_address: SocketAddr,
    proxy_address: SocketAddr,
    connection_events: &mpsc::UnboundedSender<ConnectionEvent>,
    connection_ids: &AtomicU64,
) -> Response<ProxyBody> {
    if !authorized(request.headers(), credentials) {
        let mut response = error_response(
            StatusCode::PROXY_AUTHENTICATION_REQUIRED,
            "Proxy authentication required",
        );
        response.headers_mut().insert(
            PROXY_AUTHENTICATE,
            HeaderValue::from_static("Basic realm=\"HTTP Proxy\", charset=\"UTF-8\""),
        );
        return response;
    }
    if request.method() == Method::CONNECT {
        return connect_tunnel(
            &mut request,
            jobs,
            client_address,
            proxy_address,
            connection_events,
            connection_ids,
        )
        .await;
    }
    match forward_request(
        request,
        jobs,
        client_address,
        proxy_address,
        connection_events,
        connection_ids,
    )
    .await
    {
        Ok(response) => response,
        Err((status, error)) => {
            log::debug!("http_proxy_forward_failed error={error}");
            error_response(status, &error)
        }
    }
}

fn authorized(headers: &HeaderMap, credentials: Option<&(String, String)>) -> bool {
    let Some((username, password)) = credentials else {
        return true;
    };
    let mut values = headers.get_all(PROXY_AUTHORIZATION).iter();
    let Some(value) = values.next().and_then(|value| value.to_str().ok()) else {
        return false;
    };
    if values.next().is_some() {
        return false;
    }
    let Some((scheme, encoded)) = value.split_once(' ') else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("Basic") {
        return false;
    }
    let Ok(decoded) = STANDARD.decode(encoded.trim()) else {
        return false;
    };
    let expected = format!("{username}:{password}");
    // Compare the whole credential without stopping at the first unequal byte.
    decoded.len() == expected.len()
        && decoded
            .iter()
            .zip(expected.as_bytes())
            .fold(0u8, |diff, (left, right)| diff | (left ^ right))
            == 0
}

async fn connect_tunnel(
    request: &mut Request<Incoming>,
    jobs: &mpsc::UnboundedSender<ClientJob>,
    client_address: SocketAddr,
    proxy_address: SocketAddr,
    connection_events: &mpsc::UnboundedSender<ConnectionEvent>,
    connection_ids: &AtomicU64,
) -> Response<ProxyBody> {
    let Some(authority) = request.uri().authority() else {
        return error_response(StatusCode::BAD_REQUEST, "CONNECT requires host:port");
    };
    let Ok(Some(port)) = parse_authority_port(authority.as_str()) else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "CONNECT requires a valid explicit port",
        );
    };
    if authority.as_str().contains('@')
        || request.uri().scheme().is_some()
        || request.uri().path_and_query().is_some()
    {
        return error_response(StatusCode::BAD_REQUEST, "Invalid CONNECT authority");
    }
    let host = unbracket_host(authority.host());
    let mut target = match connect_target(host, port).await {
        Ok(target) => target,
        Err((status, error)) => {
            log::debug!("http_proxy_connect_failed error={error}");
            return error_response(status, &error);
        }
    };
    let destination = match target.peer_addr() {
        Ok(destination) => destination,
        Err(error) => {
            return error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Could not determine target address: {error}"),
            );
        }
    };
    let connection = HttpProxyConnection {
        id: connection_ids.fetch_add(1, Ordering::Relaxed),
        client: client_address,
        proxy: proxy_address,
        destination,
    };
    let events = connection_events.clone();
    let upgrade = hyper::upgrade::on(request);
    let job = ClientJob::Tunnel(Box::pin(async move {
        let _connection_status = ConnectionStatusGuard::new(events, connection);
        match timeout(CONNECT_TIMEOUT, upgrade).await {
            Ok(Ok(upgraded)) => {
                let mut client = TokioIo::new(upgraded);
                if let Err(error) = tokio::io::copy_bidirectional(&mut client, &mut target).await {
                    log::debug!("http_proxy_tunnel_closed error={error}");
                }
            }
            Ok(Err(error)) => log::debug!("http_proxy_upgrade_failed error={error}"),
            Err(_) => log::debug!("http_proxy_upgrade_timeout"),
        }
    }));
    if jobs.send(job).is_err() {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "Proxy connection closed");
    }
    Response::new(empty_body())
}

async fn forward_request(
    mut request: Request<Incoming>,
    jobs: &mpsc::UnboundedSender<ClientJob>,
    client_address: SocketAddr,
    proxy_address: SocketAddr,
    connection_events: &mpsc::UnboundedSender<ConnectionEvent>,
    connection_ids: &AtomicU64,
) -> Result<Response<ProxyBody>, (StatusCode, String)> {
    let uri = request.uri().clone();
    if uri.scheme_str() != Some("http") {
        return Err((
            StatusCode::BAD_REQUEST,
            "An absolute http URI is required; use CONNECT for HTTPS".to_owned(),
        ));
    }
    let authority = uri
        .authority()
        .filter(|authority| !authority.as_str().contains('@'))
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid target authority".to_owned(),
            )
        })?;
    let port = parse_authority_port(authority.as_str())
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_owned()))?
        .unwrap_or(80);
    let target = connect_target(unbracket_host(authority.host()), port).await?;
    let destination = target.peer_addr().map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Could not determine target address: {error}"),
        )
    })?;
    let host_header = HeaderValue::from_str(authority.as_str())
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid target host".to_owned()))?;
    *request.uri_mut() = Uri::builder()
        .path_and_query(
            uri.path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/"),
        )
        .build()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid target path".to_owned()))?;
    strip_hop_headers(request.headers_mut());
    request.headers_mut().insert(HOST, host_header);
    // Each request owns one upstream connection; its body streams unchanged.
    request
        .headers_mut()
        .insert(CONNECTION, HeaderValue::from_static("close"));
    let (mut sender, upstream_connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(target))
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Upstream HTTP handshake failed: {error}"),
                )
            })?;
    let connection = HttpProxyConnection {
        id: connection_ids.fetch_add(1, Ordering::Relaxed),
        client: client_address,
        proxy: proxy_address,
        destination,
    };
    let events = connection_events.clone();
    jobs.send(ClientJob::Upstream(Box::pin(async move {
        let _connection_status = ConnectionStatusGuard::new(events, connection);
        if let Err(error) = upstream_connection.await {
            log::debug!("http_proxy_upstream_closed error={error}");
        }
    })))
    .map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Proxy connection closed".to_owned(),
        )
    })?;
    let mut response = sender.send_request(request).await.map_err(|error| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Upstream request failed: {error}"),
        )
    })?;
    strip_hop_headers(response.headers_mut());
    Ok(response.map(BodyExt::boxed_unsync))
}

async fn connect_target(host: &str, port: u16) -> Result<TcpStream, (StatusCode, String)> {
    match timeout(CONNECT_TIMEOUT, TcpStream::connect((host, port))).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) => Err((
            StatusCode::BAD_GATEWAY,
            format!("Target connection failed: {error}"),
        )),
        Err(_) => Err((
            StatusCode::GATEWAY_TIMEOUT,
            "Target connection timed out".to_owned(),
        )),
    }
}

fn parse_authority_port(authority: &str) -> Result<Option<u16>, &'static str> {
    // Authority::port() returns None for both absent and invalid ports. Inspect
    // the raw suffix so an invalid explicit port cannot become the default 80.
    let port_text = if authority.starts_with('[') {
        let (_, suffix) = authority
            .split_once(']')
            .ok_or("Invalid target authority")?;
        if suffix.is_empty() {
            return Ok(None);
        }
        Some(suffix.strip_prefix(':').ok_or("Invalid target authority")?)
    } else {
        authority.split_once(':').map(|(_, port)| port)
    };
    let Some(port_text) = port_text else {
        return Ok(None);
    };
    if port_text.is_empty() || !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("Invalid target port");
    }
    let port = port_text
        .parse::<u16>()
        .map_err(|_| "Invalid target port")?;
    if port == 0 {
        return Err("Invalid target port");
    }
    Ok(Some(port))
}

fn strip_hop_headers(headers: &mut HeaderMap) {
    let connection_headers: Vec<HeaderName> = headers
        .get_all(CONNECTION)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|name| HeaderName::from_bytes(name.trim().as_bytes()).ok())
        .collect();
    for name in connection_headers {
        headers.remove(name);
    }
    for name in [
        "connection",
        "proxy-connection",
        "proxy-authorization",
        "proxy-authenticate",
        "keep-alive",
        "te",
        "trailer",
        "transfer-encoding",
        "upgrade",
    ] {
        headers.remove(name);
    }
}

fn unbracket_host(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host)
}

fn empty_body() -> ProxyBody {
    Full::new(Bytes::new())
        .map_err(|never: Infallible| match never {})
        .boxed_unsync()
}

fn error_response(status: StatusCode, message: &str) -> Response<ProxyBody> {
    let body = Full::new(Bytes::copy_from_slice(message.as_bytes()))
        .map_err(|never: Infallible| match never {})
        .boxed_unsync();
    let mut response = Response::new(body);
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONNECTION, HeaderValue::from_static("close"));
    response
}
