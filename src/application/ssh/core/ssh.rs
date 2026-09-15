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

    pub(crate) async fn run_ssh_session(
        workspace_id: String,
        profile: SessionProfile,
        command_tx: mpsc::UnboundedSender<TerminalSessionCommand>,
        commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) {
        log::debug!(
            "SSH runtime started: workspace_id={workspace_id}, host={}, port={}",
            profile.host,
            profile.port
        );
        if let Err(error) = run(&workspace_id, &profile, command_tx, commands, model.clone()).await
        {
            log::warn!(
                "SSH runtime failed: workspace_id={workspace_id}, host={}, port={}, error={error:#}",
                profile.host,
                profile.port
            );
            model.set_status(TerminalStatus::Failed, Some(format!("{error:#}")));
        }
    }

    async fn run(
        workspace_id: &str,
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
            let key_path = path.to_owned();
            let key = tokio::task::spawn_blocking({
                let key_path = key_path.clone();
                move || russh::keys::load_secret_key(&key_path, None)
            })
            .await
            .context("加载 SSH 私钥任务失败")?
            .with_context(|| format!("加载 SSH 私钥失败: {key_path}"))?;
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
        log::debug!(
            "SSH shell connected: workspace_id={workspace_id}, host={}, port={}",
            profile.host,
            profile.port
        );
        model.set_status(TerminalStatus::Connected, None);

        let (reader, writer) = channel.split();
        let exit_message = run_connected_terminal_session(
            workspace_id,
            reader,
            writer,
            command_tx,
            commands,
            model.clone(),
        )
        .await?;

        if let Err(error) = session
            .disconnect(Disconnect::ByApplication, "", "zh-CN")
            .await
        {
            log::debug!(
                "SSH session disconnect cleanup failed: workspace_id={workspace_id}, error={error:#}"
            );
        }
        model.set_status(
            TerminalStatus::Disconnected,
            exit_message.or_else(|| Some("SSH 连接已断开".into())),
        );
        log::debug!(
            "SSH runtime stopped: workspace_id={workspace_id}, host={}, port={}, revision={}, update_revision={}",
            profile.host,
            profile.port,
            model.revision(),
            model.update_revision()
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

    pub(crate) async fn run_connected_terminal_session(
        workspace_id: &str,
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
        let mut frame_dirty = false;
        let mut content_dirty = false;
        let mut exit_message = None;

        let stop_reason = loop {
            tokio::select! {
                biased;
                command = commands.recv() => {
                    if frame_dirty {
                        publish_model(
                            &mut buffer,
                            &model,
                            &mut last_frame,
                            &status,
                            &message,
                            content_dirty,
                        );
                        frame_dirty = false;
                        content_dirty = false;
                    }
                    if !apply_command(
                        command,
                        &mut buffer,
                        &writer,
                        &model,
                        &mut frame_dirty,
                    ).await? {
                        log::debug!(
                            "SSH terminal runtime stopping: workspace_id={workspace_id}, reason=application_command"
                        );
                        break "application_command";
                    }
                }
                _ = refresh.tick() => {
                    if frame_dirty {
                        publish_model(
                            &mut buffer,
                            &model,
                            &mut last_frame,
                            &status,
                            &message,
                            content_dirty,
                        );
                        frame_dirty = false;
                        content_dirty = false;
                    }
                }
                message = reader.wait() => match message {
                    Some(ChannelMsg::Data { data })
                    | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        buffer.process(&data);
                        frame_dirty = true;
                        content_dirty = true;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        log::debug!(
                            "SSH remote shell exit status received: workspace_id={workspace_id}, exit_status={exit_status}"
                        );
                        exit_message = Some(format!(
                            "远程 Shell 已退出（状态码 {exit_status}）"
                        ));
                    }
                    Some(ChannelMsg::Eof) => {
                        log::warn!(
                            "SSH terminal channel ended: workspace_id={workspace_id}, reason=remote_eof"
                        );
                        break "remote_eof";
                    }
                    Some(ChannelMsg::Close) => {
                        log::warn!(
                            "SSH terminal channel ended: workspace_id={workspace_id}, reason=remote_close"
                        );
                        break "remote_close";
                    }
                    None => {
                        log::warn!(
                            "SSH terminal channel reader ended: workspace_id={workspace_id}, reason=reader_closed"
                        );
                        break "reader_closed";
                    }
                    _ => {}
                }
            }
        };

        if frame_dirty {
            publish_model(
                &mut buffer,
                &model,
                &mut last_frame,
                &status,
                &message,
                content_dirty,
            );
        }
        log::debug!(
            "SSH terminal runtime ended: workspace_id={workspace_id}, reason={}, revision={}, update_revision={}",
            stop_reason,
            model.revision(),
            model.update_revision()
        );
        Ok(exit_message)
    }

    async fn apply_command(
        command: Option<TerminalSessionCommand>,
        buffer: &mut TerminalBuffer,
        writer: &ChannelWriteHalf<client::Msg>,
        model: &TerminalModel,
        frame_dirty: &mut bool,
    ) -> Result<bool> {
        match command {
            Some(TerminalSessionCommand::Input(data)) => {
                if !data.is_empty() {
                    writer.data_bytes(data).await.context("发送终端输入失败")?;
                    model.mark_input();
                }
            }
            Some(TerminalSessionCommand::Resize { columns, rows }) => {
                buffer.resize(columns, rows);
                writer
                    .window_change(columns.max(1), rows.max(1), 0, 0)
                    .await
                    .context("调整 PTY 大小失败")?;
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::Scroll { lines }) => {
                buffer.scroll(lines);
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::ScrollTo { offset }) => {
                buffer.scroll_to(offset);
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::Read {
                offset,
                limit,
                since_revision,
                reply,
            }) => {
                let mut page = buffer.read_text(offset, limit);
                page.revision = model.revision();
                page.changed = since_revision.map_or(true, |revision| revision != page.revision);
                if !page.changed {
                    page.text.clear();
                    page.limit = 0;
                    page.has_more = false;
                }
                let _ = reply.send(page);
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
        content_changed: bool,
    ) {
        let next_frame = Arc::new(buffer.frame_reusing(Some(last_frame.as_ref())));
        if same_frame(last_frame, &next_frame) {
            return;
        }
        if last_frame.cursor != next_frame.cursor {
            // log::debug!(
            //     "终端光标状态变化: previous={:?}, current={:?}",
            //     last_frame.cursor,
            //     next_frame.cursor
            // );
        }
        *last_frame = next_frame;
        let data = TerminalData {
            frame: last_frame.clone(),
            status: status.clone(),
            message: message.clone(),
        };
        if content_changed {
            model.replace(data);
        } else {
            model.replace_view(data);
        }
    }

    fn same_frame(left: &TerminalFrame, right: &TerminalFrame) -> bool {
        left.application_cursor == right.application_cursor
            && left.cursor == right.cursor
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

pub(crate) use core::run_ssh_session;

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
        let endpoint = self.endpoint.clone();
        let public_key = server_public_key.clone();
        match tokio::task::spawn_blocking(move || verify_host_key(&endpoint, &public_key)).await {
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
