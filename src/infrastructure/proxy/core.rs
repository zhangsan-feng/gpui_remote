use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use russh::client;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinSet,
};
use tokio_socks::tcp::Socks5Stream;

use super::{
    ConnectionAdapter, ConnectionFuture, DirectAdapter, LocalPortForward, Socks5Adapter,
    SshTunnelAdapter, SshTunnelSettings,
};

impl ConnectionAdapter for DirectAdapter {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a> {
        Box::pin(async move {
            let stream = TcpStream::connect(target)
                .await
                .with_context(|| format!("连接目标 {}:{} 失败", target.0, target.1))?;
            Ok(Box::new(stream) as Box<dyn super::ProxyStream>)
        })
    }
}

impl ConnectionAdapter for Socks5Adapter {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a> {
        Box::pin(async move {
            let proxy = &self.settings;
            let proxy_address = (proxy.host.as_str(), proxy.port);
            let stream = if proxy.username.is_empty() {
                Socks5Stream::connect(proxy_address, target).await
            } else {
                Socks5Stream::connect_with_password(
                    proxy_address,
                    target,
                    &proxy.username,
                    &proxy.password,
                )
                .await
            }
            .with_context(|| format!("连接 SOCKS5 代理 {}:{} 失败", proxy.host, proxy.port))?;
            Ok(Box::new(stream) as Box<dyn super::ProxyStream>)
        })
    }
}

impl SshTunnelAdapter {
    /// Starts a loopback listener on an available local port.
    pub async fn bind_local(&self, target: (&str, u16)) -> Result<LocalPortForward> {
        self.bind_local_at(SocketAddr::from(([127, 0, 0, 1], 0)), target)
            .await
    }

    /// Starts a local listener at `bind_addr` and forwards connections to the
    /// remote target through SSH. Binding to loopback is recommended.
    pub async fn bind_local_at(
        &self,
        bind_addr: SocketAddr,
        target: (&str, u16),
    ) -> Result<LocalPortForward> {
        let listener = TcpListener::bind(bind_addr)
            .await
            .with_context(|| format!("绑定 SSH 本地转发地址 {bind_addr} 失败"))?;
        let local_addr = listener.local_addr().context("读取 SSH 本地转发地址失败")?;
        let (shutdown, shutdown_rx) = oneshot::channel();
        let adapter = self.clone();
        let target_host = target.0.to_owned();
        let target_port = target.1;
        let task = tokio::spawn(run_local_port_forward(
            listener,
            adapter,
            target_host,
            target_port,
            shutdown_rx,
        ));

        log::debug!(
            "SSH local port forward started: local_addr={}, target_host={}, target_port={}",
            local_addr,
            target.0,
            target.1
        );
        Ok(LocalPortForward::new(local_addr, shutdown, task))
    }
}

impl ConnectionAdapter for SshTunnelAdapter {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a> {
        let settings = self.settings.clone();
        let target_host = target.0.to_owned();
        Box::pin(async move { connect_through_ssh_tunnel(&settings, &target_host, target.1).await })
    }
}

async fn connect_through_ssh_tunnel(
    settings: &SshTunnelSettings,
    target_host: &str,
    target_port: u16,
) -> Result<Box<dyn super::ProxyStream>> {
    let endpoint = format!("[{}]:{}", settings.host, settings.port);
    log::debug!(
        "SSH tunnel connecting: jump_host={}, jump_port={}, target_host={}, target_port={}",
        settings.host,
        settings.port,
        target_host,
        target_port
    );
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    });
    let mut session = client::connect(
        config,
        (settings.host.as_str(), settings.port),
        SshTunnelClientHandler { endpoint },
    )
    .await
    .context("连接 SSH 隧道跳板机失败")?;

    let authentication = if let Some(path) = settings.private_key_path.as_deref() {
        let key_path = path.to_owned();
        let key = tokio::task::spawn_blocking({
            let key_path = key_path.clone();
            move || russh::keys::load_secret_key(&key_path, None)
        })
        .await
        .context("加载 SSH 隧道私钥任务失败")?
        .with_context(|| format!("加载 SSH 隧道私钥失败: {key_path}"))?;
        session
            .authenticate_publickey(
                settings.username.clone(),
                russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
            )
            .await
            .context("SSH 隧道私钥认证失败")?
    } else {
        session
            .authenticate_password(settings.username.clone(), settings.password.clone())
            .await
            .context("SSH 隧道密码认证失败")?
    };
    if !authentication.success() {
        bail!("SSH 隧道跳板机认证失败");
    }

    let channel = session
        .channel_open_direct_tcpip(target_host.to_owned(), target_port.into(), "127.0.0.1", 0)
        .await
        .with_context(|| format!("打开 SSH 隧道目标 {}:{} 失败", target_host, target_port))?;
    log::debug!(
        "SSH tunnel connected: jump_host={}, target_host={}, target_port={}",
        settings.host,
        target_host,
        target_port
    );

    Ok(Box::new(channel.into_stream()))
}

async fn run_local_port_forward(
    listener: TcpListener,
    adapter: SshTunnelAdapter,
    target_host: String,
    target_port: u16,
    mut shutdown: oneshot::Receiver<()>,
) {
    let mut connections = JoinSet::new();

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                stop_forward_connections(&mut connections).await;
                log::debug!(
                    "SSH local port forward stopped: target_host={}, target_port={}",
                    target_host,
                    target_port
                );
                return;
            }
            Some(result) = connections.join_next(), if !connections.is_empty() => {
                if let Err(error) = result {
                    log::debug!("SSH local port forward task ended: {error}");
                }
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((local_stream, peer_addr)) => {
                        let adapter = adapter.clone();
                        let target_host = target_host.clone();
                        connections.spawn(async move {
                            if let Err(error) = forward_local_connection(
                                local_stream,
                                adapter,
                                target_host,
                                target_port,
                            )
                            .await
                            {
                                log::debug!(
                                    "SSH local port forward connection failed: peer={}, error={error:#}",
                                    peer_addr
                                );
                            }
                        });
                    }
                    Err(error) => {
                        log::warn!("SSH local port forward accept failed: {error}");
                        stop_forward_connections(&mut connections).await;
                        return;
                    }
                }
            }
        }
    }
}

async fn stop_forward_connections(connections: &mut JoinSet<()>) {
    connections.abort_all();
    while connections.join_next().await.is_some() {}
}

async fn forward_local_connection(
    mut local_stream: TcpStream,
    adapter: SshTunnelAdapter,
    target_host: String,
    target_port: u16,
) -> Result<()> {
    let mut remote_stream = adapter
        .connect((&target_host, target_port))
        .await
        .with_context(|| format!("连接 SSH 隧道目标 {target_host}:{target_port} 失败"))?;
    tokio::io::copy_bidirectional(&mut local_stream, &mut remote_stream)
        .await
        .with_context(|| format!("转发 SSH 隧道数据到 {target_host}:{target_port} 失败"))?;
    Ok(())
}

struct SshTunnelClientHandler {
    endpoint: String,
}

impl client::Handler for SshTunnelClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let endpoint = self.endpoint.clone();
        let public_key = server_public_key.public_key();
        match tokio::task::spawn_blocking(move || {
            crate::infrastructure::storage::verify_host_key(&endpoint, &public_key)
        })
        .await
        {
            Ok(Ok(accepted)) => Ok(accepted),
            Ok(Err(error)) => {
                log::info!("SSH tunnel host key verification failed: {error:#}");
                Ok(false)
            }
            Err(error) => {
                log::info!("SSH tunnel host key verification task failed: {error}");
                Ok(false)
            }
        }
    }
}

pub(super) async fn connect(
    target: (&str, u16),
    proxy: Option<&super::ProxySettings>,
) -> Result<Box<dyn super::ProxyStream>> {
    match proxy {
        Some(proxy) => Socks5Adapter::new(proxy.clone()).connect(target).await,
        None => DirectAdapter.connect(target).await,
    }
}
