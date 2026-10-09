use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect, client};

use crate::{
    application::ports::{
        SshChannelEvent, SshOpenFuture, SshResultFuture, SshShell, SshWaitFuture,
    },
    domain::session::SessionProfile,
};

use crate::infrastructure::connection::ConnectionPort;

const DEFAULT_COLUMNS: u32 = 120;
const DEFAULT_ROWS: u32 = 36;

struct ClientHandler {
    endpoint: String,
}

impl client::Handler for ClientHandler {
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
                log::info!("SSH host key verification failed: {error:#}");
                Ok(false)
            }
            Err(error) => {
                log::info!("SSH host key verification task failed: {error}");
                Ok(false)
            }
        }
    }
}

pub(super) fn open_shell<'a>(
    connection: &'a dyn ConnectionPort,
    workspace_id: &'a str,
    profile: &'a SessionProfile,
) -> SshOpenFuture<'a> {
    Box::pin(async move {
        let endpoint = format!("[{}]:{}", profile.host, profile.port);
        log::debug!(
            "SSH infrastructure connecting: workspace_id={workspace_id}, host={}, port={}, proxy_enabled={}",
            profile.host,
            profile.port,
            profile.proxy.is_some()
        );
        let stream = connection
            .connect(profile.host.as_str(), profile.port, profile.proxy.as_ref())
            .await?;
        log::debug!("SSH TCP transport connected: workspace_id={workspace_id}");
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });
        let mut session = client::connect_stream(config, stream, ClientHandler { endpoint })
            .await
            .context("SSH 握手或主机密钥校验失败")?;
        log::debug!("SSH protocol handshake completed: workspace_id={workspace_id}");

        let (authentication, authentication_method) =
            if let Some(path) = profile.private_key_path.as_deref() {
                let key_path = path.to_owned();
                let key = tokio::task::spawn_blocking({
                    let key_path = key_path.clone();
                    move || russh::keys::load_secret_key(&key_path, None)
                })
                .await
                .context("加载 SSH 私钥任务失败")?
                .with_context(|| format!("加载 SSH 私钥失败: {key_path}"))?;
                (
                    session
                        .authenticate_publickey(
                            profile.username.clone(),
                            russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                        )
                        .await
                        .context("SSH 私钥认证失败")?,
                    "public_key",
                )
            } else {
                (
                    session
                        .authenticate_password(profile.username.clone(), profile.password.clone())
                        .await
                        .context("SSH 密码认证失败")?,
                    "password",
                )
            };
        if !authentication.success() {
            if profile.private_key_path.is_some() {
                bail!("SSH 用户名或私钥错误");
            }
            bail!("SSH 用户名或密码错误");
        }
        log::debug!(
            "SSH authentication succeeded: workspace_id={workspace_id}, method={authentication_method}"
        );

        let channel = session
            .channel_open_session()
            .await
            .context("创建 SSH 会话通道失败")?;
        log::debug!("SSH session channel opened: workspace_id={workspace_id}");
        channel
            .request_pty(
                true,
                "xterm-256color",
                DEFAULT_COLUMNS,
                DEFAULT_ROWS,
                0,
                0,
                &[],
            )
            .await
            .context("申请远程 PTY 失败")?;
        log::debug!(
            "SSH PTY request completed: workspace_id={workspace_id}, columns={DEFAULT_COLUMNS}, rows={DEFAULT_ROWS}"
        );
        channel
            .request_shell(true)
            .await
            .context("启动远程 Shell 失败")?;
        log::debug!("SSH shell request completed: workspace_id={workspace_id}");
        let (reader, writer) = channel.split();
        Ok(Box::new(RusshShell {
            session,
            reader: tokio::sync::Mutex::new(reader),
            writer,
        }) as Box<dyn SshShell>)
    })
}

struct RusshShell {
    session: client::Handle<ClientHandler>,
    reader: tokio::sync::Mutex<ChannelReadHalf>,
    writer: ChannelWriteHalf<client::Msg>,
}

impl SshShell for RusshShell {
    fn wait_event(&self) -> SshWaitFuture<'_> {
        Box::pin(async move {
            self.reader
                .lock()
                .await
                .wait()
                .await
                .map(|message| match message {
                    ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                        SshChannelEvent::Data(data.to_vec())
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        SshChannelEvent::ExitStatus(exit_status)
                    }
                    ChannelMsg::ExitSignal {
                        signal_name,
                        core_dumped,
                        error_message,
                        ..
                    } => SshChannelEvent::ExitSignal {
                        signal_name: format!("{signal_name:?}"),
                        core_dumped,
                        error_message: format!("{error_message:?}"),
                    },
                    ChannelMsg::Eof => SshChannelEvent::Eof,
                    ChannelMsg::Close => SshChannelEvent::Close,
                    _ => SshChannelEvent::Other,
                })
        })
    }

    fn send_data(&self, data: Vec<u8>) -> SshResultFuture<'_> {
        Box::pin(async move {
            self.writer
                .data_bytes(data)
                .await
                .context("发送终端输入失败")
        })
    }

    fn resize(&self, columns: u32, rows: u32) -> SshResultFuture<'_> {
        Box::pin(async move {
            self.writer
                .window_change(columns.max(1), rows.max(1), 0, 0)
                .await
                .context("调整 PTY 大小失败")
        })
    }

    fn close_channel(&self) -> SshResultFuture<'_> {
        Box::pin(async move {
            let eof = self.writer.eof().await.context("发送 SSH channel EOF 失败");
            let close = self.writer.close().await.context("关闭 SSH channel 失败");
            eof.and(close)
        })
    }

    fn disconnect(&self) -> SshResultFuture<'_> {
        Box::pin(async move {
            self.session
                .disconnect(Disconnect::ByApplication, "", "zh-CN")
                .await
                .context("关闭 SSH transport 失败")
        })
    }
}
