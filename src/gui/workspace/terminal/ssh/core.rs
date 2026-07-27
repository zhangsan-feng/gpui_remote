use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use russh::{Disconnect, client};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_socks::tcp::Socks5Stream;

use crate::domain::{
    session::SessionProfile,
    terminal::{TerminalSessionCommand, TerminalStatus},
};

use super::{
    super::pty::TerminalModel, ClientHandler, DEFAULT_COLUMNS, DEFAULT_ROWS, SshStream,
    runtime::run_connected_terminal_session,
};

pub(in crate::gui::workspace::terminal) async fn run_ssh_session(
    profile: SessionProfile,
    command_tx: mpsc::UnboundedSender<TerminalSessionCommand>,
    commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
    model: Arc<TerminalModel>,
) {
    if let Err(error) = run(&profile, command_tx, commands, model.clone()).await {
        model.set_status(TerminalStatus::Failed, Some(format!("{error:#}")));
    }
}

async fn run(
    profile: &SessionProfile,
    command_tx: mpsc::UnboundedSender<TerminalSessionCommand>,
    commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
    model: Arc<TerminalModel>,
) -> Result<()> {
    let stream = connect_transport(profile).await?;
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    });
    let mut session = client::connect_stream(
        config,
        stream,
        ClientHandler {
            endpoint: format!("[{}]:{}", profile.host, profile.port),
        },
    )
    .await
    .context("SSH 握手或主机密钥校验失败")?;
    let authentication = session
        .authenticate_password(profile.username.clone(), profile.password.clone())
        .await
        .context("SSH 密码认证失败")?;
    if !authentication.success() {
        bail!("SSH 用户名或密码错误");
    }

    let channel = session
        .channel_open_session()
        .await
        .context("创建 SSH 会话通道失败")?;
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
    channel
        .request_shell(true)
        .await
        .context("启动远程 Shell 失败")?;
    model.set_status(TerminalStatus::Connected, None);

    let (reader, writer) = channel.split();
    let exit_message =
        run_connected_terminal_session(reader, writer, command_tx, commands, model.clone()).await?;

    let _ = session
        .disconnect(Disconnect::ByApplication, "", "zh-CN")
        .await;
    model.set_status(
        TerminalStatus::Disconnected,
        exit_message.or_else(|| Some("SSH 连接已断开".into())),
    );
    Ok(())
}

async fn connect_transport(profile: &SessionProfile) -> Result<Box<dyn SshStream>> {
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
            .with_context(|| format!("连接 SSH 主机 {}:{} 失败", profile.host, profile.port))?;
        Ok(Box::new(stream))
    }
}
