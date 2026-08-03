use std::time::Duration;

use anyhow::{Context as _, Result};
use russh::client;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_socks::tcp::Socks5Stream;

use crate::{domain::session::SessionProfile, infrastructure::storage::verify_host_key};

pub(super) trait SshStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> SshStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

pub(super) struct SftpClientHandler {
    pub(super) endpoint: String,
}

impl client::Handler for SftpClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match verify_host_key(&self.endpoint, server_public_key) {
            Ok(accepted) => Ok(accepted),
            Err(error) => {
                log::info!("SFTP host key verification failed: {error:#}");
                Ok(false)
            }
        }
    }
}

pub(super) async fn connect_transport(profile: &SessionProfile) -> Result<Box<dyn SshStream>> {
    let target = (profile.host.as_str(), profile.port);
    if let Some(proxy) = &profile.proxy {
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
        Ok(Box::new(stream))
    } else {
        let stream = TcpStream::connect(target)
            .await
            .with_context(|| format!("连接 SFTP 主机 {}:{} 失败", profile.host, profile.port))?;
        Ok(Box::new(stream))
    }
}

pub(super) fn ssh_config() -> client::Config {
    client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    }
}
