mod core {
    use std::{sync::Arc, time::Duration};

    use anyhow::{Context as _, Result, bail};
    use russh::{Disconnect, client};
    use tokio::sync::mpsc;

    use crate::{
        domain::{
            session::SessionProfile,
            terminal::{TerminalSessionCommand, TerminalStatus},
        },
        infrastructure::proxy::{ProxySettings, connect},
    };

    use super::{
        super::pty::TerminalModel, ClientHandler, DEFAULT_COLUMNS, DEFAULT_ROWS,
        runtime::run_connected_terminal_session,
    };

    pub(in crate::gui::workspace::ssh) async fn run_ssh_session(
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
        let proxy = profile.proxy.as_ref().map(|proxy| ProxySettings {
            host: proxy.host.clone(),
            port: proxy.port,
            username: proxy.username.clone(),
            password: proxy.password.clone(),
        });
        let stream = connect((profile.host.as_str(), profile.port), proxy.as_ref()).await?;
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
        let authentication = if let Some(path) = profile.private_key_path.as_deref() {
            let key = russh::keys::load_secret_key(path, None)
                .with_context(|| format!("加载 SSH 私钥失败: {path}"))?;
            session
                .authenticate_publickey(
                    profile.username.clone(),
                    russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await
                .context("SSH 私钥认证失败")?
        } else {
            session
                .authenticate_password(profile.username.clone(), profile.password.clone())
                .await
                .context("SSH 密码认证失败")?
        };
        if !authentication.success() {
            if profile.private_key_path.is_some() {
                bail!("SSH 用户名或私钥错误");
            }
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
            run_connected_terminal_session(reader, writer, command_tx, commands, model.clone())
                .await?;

        let _ = session
            .disconnect(Disconnect::ByApplication, "", "zh-CN")
            .await;
        model.set_status(
            TerminalStatus::Disconnected,
            exit_message.or_else(|| Some("SSH 连接已断开".into())),
        );
        Ok(())
    }
}

mod runtime {
    use std::{sync::Arc, time::Duration};

    use anyhow::{Context as _, Result};
    use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, client};
    use tokio::sync::mpsc;

    use crate::domain::terminal::{
        TerminalData, TerminalFrame, TerminalSessionCommand, TerminalStatus,
    };

    use super::super::{buffer::TerminalBuffer, pty::TerminalModel};

    const REFRESH_INTERVAL: Duration = Duration::from_millis(33);

    pub(super) async fn run_connected_terminal_session(
        mut reader: ChannelReadHalf,
        writer: ChannelWriteHalf<client::Msg>,
        command_tx: mpsc::UnboundedSender<TerminalSessionCommand>,
        mut commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) -> Result<Option<String>> {
        let mut buffer = TerminalBuffer::new(command_tx);
        let mut refresh = tokio::time::interval(REFRESH_INTERVAL);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        refresh.tick().await;

        let (mut last_frame, status, message) = {
            let data = model.read();
            (
                data.frame.clone(),
                data.status.clone(),
                data.message.clone(),
            )
        };
        let mut dirty = false;
        let mut exit_message = None;

        loop {
            tokio::select! {
                biased;
                command = commands.recv() => {
                    if !apply_command(command, &mut buffer, &writer, &mut dirty).await? {
                        break;
                    }
                }
                _ = refresh.tick() => {
                    if dirty {
                        publish_model(
                            &mut buffer,
                            &model,
                            &mut last_frame,
                            &status,
                            &message,
                        );
                        dirty = false;
                    }
                }
                message = reader.wait() => match message {
                    Some(ChannelMsg::Data { data })
                    | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        buffer.process(&data);
                        dirty = true;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        exit_message = Some(format!(
                            "远程 Shell 已退出（状态码 {exit_status}）"
                        ));
                    }
                    Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }

        if dirty {
            publish_model(&mut buffer, &model, &mut last_frame, &status, &message);
        }
        Ok(exit_message)
    }

    async fn apply_command(
        command: Option<TerminalSessionCommand>,
        buffer: &mut TerminalBuffer,
        writer: &ChannelWriteHalf<client::Msg>,
        dirty: &mut bool,
    ) -> Result<bool> {
        match command {
            Some(TerminalSessionCommand::Input(data)) => {
                writer.data_bytes(data).await.context("发送终端输入失败")?;
            }
            Some(TerminalSessionCommand::Resize { columns, rows }) => {
                buffer.resize(columns, rows);
                writer
                    .window_change(columns.max(1), rows.max(1), 0, 0)
                    .await
                    .context("调整 PTY 大小失败")?;
                *dirty = true;
            }
            Some(TerminalSessionCommand::Scroll { lines }) => {
                buffer.scroll(lines);
                *dirty = true;
            }
            Some(TerminalSessionCommand::ScrollTo { offset }) => {
                buffer.scroll_to(offset);
                *dirty = true;
            }
            Some(TerminalSessionCommand::Read {
                offset,
                limit,
                reply,
            }) => {
                let _ = reply.send(buffer.read_text(offset, limit));
            }
            Some(TerminalSessionCommand::Disconnect) | None => {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn publish_model(
        buffer: &mut TerminalBuffer,
        model: &TerminalModel,
        last_frame: &mut Arc<TerminalFrame>,
        status: &TerminalStatus,
        message: &Option<String>,
    ) {
        let next_frame = Arc::new(buffer.frame_reusing(Some(last_frame.as_ref())));
        if same_frame(last_frame, &next_frame) {
            return;
        }
        *last_frame = next_frame;
        model.replace(TerminalData {
            frame: last_frame.clone(),
            status: status.clone(),
            message: message.clone(),
        });
    }

    fn same_frame(left: &TerminalFrame, right: &TerminalFrame) -> bool {
        left.application_cursor == right.application_cursor
            && left.history_size == right.history_size
            && left.display_offset == right.display_offset
            && left.lines.len() == right.lines.len()
            && left
                .lines
                .iter()
                .zip(right.lines.iter())
                .all(|(left, right)| Arc::ptr_eq(left, right))
    }
}

use crate::infrastructure::storage::verify_host_key;

pub(super) use core::run_ssh_session;

const DEFAULT_COLUMNS: u32 = 120;
const DEFAULT_ROWS: u32 = 36;

struct ClientHandler {
    endpoint: String,
}

impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match verify_host_key(&self.endpoint, server_public_key) {
            Ok(accepted) => {
                if !accepted {
                    log::info!("SSH host key changed for {}", self.endpoint);
                }
                Ok(accepted)
            }
            Err(error) => {
                log::info!("SSH host key verification failed: {error:#}");
                Ok(false)
            }
        }
    }
}
