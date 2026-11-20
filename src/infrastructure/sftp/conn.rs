use std::time::Duration;

use crate::infrastructure::storage::verify_host_key;
use anyhow::{Context as _, Result, bail};
use russh::{Channel, ChannelMsg, client};

pub(super) struct SftpClientHandler {
    pub(super) endpoint: String,
}

impl client::Handler for SftpClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let endpoint = self.endpoint.clone();
        let public_key = server_public_key.public_key();
        match tokio::task::spawn_blocking(move || verify_host_key(&endpoint, &public_key)).await {
            Ok(Ok(accepted)) => Ok(accepted),
            Ok(Err(error)) => {
                log::info!("SFTP host key verification failed: {error:#}");
                Ok(false)
            }
            Err(error) => {
                log::info!("SFTP host key verification task failed: {error}");
                Ok(false)
            }
        }
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

pub(super) async fn request_sftp_subsystem(
    channel: &mut Channel<client::Msg>,
    workspace_id: &str,
) -> Result<()> {
    channel
        .request_subsystem(true, "sftp")
        .await
        .context("发送远程 SFTP 子系统请求失败")?;
    // request_subsystem only queues the request. ChannelStream ignores control
    // messages, so consume the acknowledgement before starting the SFTP codec.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match channel.wait().await {
                Some(ChannelMsg::Success) => {
                    log::debug!("SFTP subsystem accepted: workspace_id={workspace_id}");
                    return Ok(());
                }
                Some(ChannelMsg::Failure) => {
                    bail!("远端拒绝启动 SFTP 子系统，请检查 SSH 服务的 Subsystem sftp 配置");
                }
                Some(ChannelMsg::Eof | ChannelMsg::Close) | None => {
                    bail!(
                        "远端在确认 SFTP 子系统前关闭了通道，请检查 SSH 服务的 Subsystem sftp 配置"
                    );
                }
                Some(ChannelMsg::ExtendedData { data, .. }) => {
                    bail!(
                        "远端启动 SFTP 子系统失败: {}",
                        String::from_utf8_lossy(&data)
                    );
                }
                Some(ChannelMsg::Data { .. }) => {
                    bail!("远端在确认 SFTP 子系统前返回了非预期数据");
                }
                Some(message) => {
                    log::debug!(
                        "SFTP subsystem pending: workspace_id={workspace_id}, message={message:?}"
                    );
                }
            }
        }
    })
    .await
    .context("等待远端确认 SFTP 子系统超时")?
}
