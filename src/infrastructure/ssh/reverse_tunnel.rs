use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use russh::{ChannelOpenFailure, Disconnect, client};
use tokio::net::TcpStream;

use crate::{
    domain::session::{ConnectionProtocol, SessionProfile},
    infrastructure::{InfrastructureContext, connection::ConnectionPort},
};

pub(crate) struct SshReverseTunnelController {
    runtimes: tokio::sync::Mutex<HashMap<String, TunnelRuntime>>,
}

struct TunnelRuntime {
    session: client::Handle<TunnelClientHandler>,
    remote_address: String,
    remote_port: u16,
}

struct TunnelClientHandler {
    endpoint: String,
    forward_address: String,
    forward_port: u16,
}

impl SshReverseTunnelController {
    pub(crate) fn new() -> Self {
        Self {
            runtimes: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    pub(crate) async fn open(
        &self,
        infrastructure: &InfrastructureContext,
        profile: &SessionProfile,
    ) -> Result<()> {
        if profile.connection_protocol != ConnectionProtocol::SshAndSftp {
            bail!("SSH 隧道仅支持 SSH/SFTP 连接");
        }
        let tunnel = profile
            .ssh_reverse_tunnel
            .as_ref()
            .context("请先在连接配置的 SSH 隧道页填写转发参数")?;
        let mut runtimes = self.runtimes.lock().await;
        if runtimes.contains_key(&profile.id) {
            return Ok(());
        }

        let endpoint = format!("[{}]:{}", profile.host, profile.port);
        log::debug!(
            "SSH reverse tunnel connecting: profile_id={}, host={}, port={}, proxy_enabled={}",
            profile.id,
            profile.host,
            profile.port,
            profile.proxy.is_some()
        );
        let stream = infrastructure
            .connect(profile.host.as_str(), profile.port, profile.proxy.as_ref())
            .await?;
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });
        let mut session = client::connect_stream(
            config,
            stream,
            TunnelClientHandler {
                endpoint,
                forward_address: tunnel.forward_address.clone(),
                forward_port: tunnel.forward_port,
            },
        )
        .await
        .context("SSH 隧道握手或主机密钥校验失败")?;

        authenticate(&mut session, profile).await?;
        session
            .tcpip_forward(profile.host.clone(), u32::from(tunnel.remote_port))
            .await
            .with_context(|| {
                format!(
                    "开启 SSH 反向隧道失败: remote={}:{}, target={}:{}",
                    profile.host, tunnel.remote_port, tunnel.forward_address, tunnel.forward_port
                )
            })?;

        log::info!(
            "SSH reverse tunnel started: profile_id={}, remote={}:{}, target={}:{}, authentication={}",
            profile.id,
            profile.host,
            tunnel.remote_port,
            tunnel.forward_address,
            tunnel.forward_port,
            if profile.private_key_path.is_some() {
                "public_key"
            } else {
                "password"
            }
        );
        runtimes.insert(
            profile.id.clone(),
            TunnelRuntime {
                session,
                remote_address: profile.host.clone(),
                remote_port: tunnel.remote_port,
            },
        );
        Ok(())
    }

    pub(crate) async fn close(&self, profile_id: &str) {
        let runtime = self.runtimes.lock().await.remove(profile_id);
        let Some(runtime) = runtime else {
            return;
        };
        match tokio::time::timeout(
            Duration::from_secs(3),
            runtime
                .session
                .cancel_tcpip_forward(runtime.remote_address, u32::from(runtime.remote_port)),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                log::debug!("SSH reverse tunnel cancellation failed for {profile_id}: {error}");
            }
            Err(_) => {
                log::debug!("SSH reverse tunnel cancellation timed out for {profile_id}");
            }
        }
        if let Err(error) = runtime
            .session
            .disconnect(Disconnect::ByApplication, "SSH tunnel closed", "zh-CN")
            .await
        {
            log::debug!("SSH reverse tunnel disconnect failed for {profile_id}: {error}");
        }
        log::info!("SSH reverse tunnel closed: profile_id={profile_id}");
    }

    pub(crate) async fn active_ids(&self) -> std::collections::HashSet<String> {
        self.runtimes.lock().await.keys().cloned().collect()
    }
}

impl client::Handler for TunnelClientHandler {
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

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        _: &str,
        _: u32,
        reply: client::ChannelOpenHandle,
        _: &mut client::Session,
    ) -> Result<(), Self::Error> {
        handle_forwarded_channel(
            channel,
            reply,
            connected_address.to_owned(),
            connected_port.min(u16::MAX as u32) as u16,
            self.forward_address.clone(),
            self.forward_port,
        );
        Ok(())
    }
}

async fn authenticate(
    session: &mut client::Handle<TunnelClientHandler>,
    profile: &SessionProfile,
) -> Result<()> {
    let (authentication, method) = if let Some(path) = profile.private_key_path.as_deref() {
        let key_path = path.to_owned();
        let key = tokio::task::spawn_blocking({
            let key_path = key_path.clone();
            move || russh::keys::load_secret_key(&key_path, None)
        })
        .await
        .context("加载 SSH 隧道私钥任务失败")?
        .with_context(|| format!("加载 SSH 隧道私钥失败: {key_path}"))?;
        (
            session
                .authenticate_publickey(
                    profile.username.clone(),
                    russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await
                .context("SSH 隧道私钥认证失败")?,
            "public_key",
        )
    } else {
        (
            session
                .authenticate_password(profile.username.clone(), profile.password.clone())
                .await
                .context("SSH 隧道密码认证失败")?,
            "password",
        )
    };
    if !authentication.success() {
        if profile.private_key_path.is_some() {
            bail!("SSH 隧道用户名或私钥错误");
        }
        bail!("SSH 隧道用户名或密码错误");
    }
    log::debug!(
        "SSH reverse tunnel authentication succeeded: profile_id={}, method={method}",
        profile.id
    );
    Ok(())
}

pub(crate) fn handle_forwarded_channel(
    channel: russh::Channel<client::Msg>,
    reply: client::ChannelOpenHandle,
    remote_address: String,
    remote_port: u16,
    forward_address: String,
    forward_port: u16,
) {
    tokio::spawn(async move {
        let mut forward_stream = match TcpStream::connect((forward_address.as_str(), forward_port))
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                log::warn!(
                    "SSH reverse tunnel local target connection failed: remote_listen={remote_address}:{remote_port}, target={forward_address}:{forward_port}, error={error}"
                );
                reply.reject(ChannelOpenFailure::ConnectFailed).await;
                return;
            }
        };

        reply.accept().await;
        let mut remote_stream = channel.into_stream();
        if let Err(error) =
            tokio::io::copy_bidirectional(&mut remote_stream, &mut forward_stream).await
        {
            log::debug!(
                "SSH reverse tunnel forwarding ended with error: remote_listen={remote_address}:{remote_port}, target={forward_address}:{forward_port}, error={error}"
            );
        }
    });
}
