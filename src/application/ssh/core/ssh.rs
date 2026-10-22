mod core {
    use std::sync::Arc;

    use anyhow::Result;
    use tokio::sync::mpsc;

    use crate::{
        domain::{
            session::SessionProfile,
            terminal::{TerminalSessionCommand, TerminalStatus},
        },
        infrastructure::INFRASTRUCTURE,
    };

    use super::{
        super::pty::TerminalModel,
        runtime::{run_closed_terminal_session, run_connected_terminal_session},
    };

    pub(crate) async fn run_ssh_session(
        workspace_id: String,
        profile: SessionProfile,
        commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) {
        log::debug!(
            "SSH runtime started: workspace_id={workspace_id}, host={}, port={}",
            profile.host,
            profile.port
        );
        if let Err(error) = run(&workspace_id, &profile, commands, model.clone()).await {
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
        commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) -> Result<()> {
        log::debug!(
            "SSH protocol adapter opening shell: workspace_id={workspace_id}, host={}, port={}, proxy_enabled={}",
            profile.host,
            profile.port,
            profile.proxy.is_some()
        );
        let shell = INFRASTRUCTURE.open_ssh_shell(workspace_id, profile).await?;
        log::debug!(
            "SSH shell connected: workspace_id={workspace_id}, host={}, port={}",
            profile.host,
            profile.port
        );
        model.set_status(TerminalStatus::Connected, None);

        let terminal_exit =
            run_connected_terminal_session(workspace_id, shell.as_ref(), commands, model.clone())
                .await?;

        log::debug!(
            "SSH session cleanup starting: workspace_id={workspace_id}, reason={}",
            terminal_exit.stop_reason
        );
        if let Err(error) = shell.disconnect().await {
            log::debug!(
                "SSH session disconnect cleanup failed: workspace_id={workspace_id}, error={error:#}"
            );
        }
        let exit_message = terminal_exit.exit_message;
        model.set_status(
            TerminalStatus::Disconnected,
            exit_message.or_else(|| Some("SSH 连接已断开".into())),
        );
        if terminal_exit.remote_closed {
            log::debug!(
                "SSH terminal history runtime started: workspace_id={workspace_id}, reason=remote_channel_closed"
            );
            run_closed_terminal_session(workspace_id, terminal_exit.commands, model.clone())
                .await?;
        }
        log::debug!(
            "SSH runtime stopped: workspace_id={workspace_id}, host={}, port={}, mcp_snapshot_version={}, gui_snapshot_version={}",
            profile.host,
            profile.port,
            model.mcp_snapshot_version(),
            model.gui_snapshot_version()
        );
        Ok(())
    }
}

mod runtime {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use anyhow::Result;
    use tokio::sync::mpsc;

    use crate::{
        application::ports::{SshChannelEvent, SshShell},
        domain::terminal::{TerminalData, TerminalFrame, TerminalSessionCommand},
    };

    use super::super::pty::TerminalModel;

    const REFRESH_INTERVAL: Duration = Duration::from_millis(33);
    const CHANNEL_CLOSE_GRACE_PERIOD: Duration = Duration::from_secs(5);
    const PERF_LOG_INTERVAL: Duration = Duration::from_secs(5);

    #[derive(Default)]
    struct TerminalPerfWindow {
        started_at: Option<Instant>,
        reported_idle: bool,
        data_chunks: u64,
        data_bytes: u64,
        process_time: Duration,
        max_process_time: Duration,
        response_count: u64,
        response_bytes: u64,
        response_send_time: Duration,
        max_response_send_time: Duration,
        command_count: u64,
        command_time: Duration,
        max_command_time: Duration,
        frame_build_count: u64,
        frame_build_time: Duration,
        max_frame_build_time: Duration,
        publish_attempts: u64,
        publish_time: Duration,
        max_publish_time: Duration,
    }

    impl TerminalPerfWindow {
        fn new() -> Self {
            Self {
                started_at: Some(Instant::now()),
                ..Self::default()
            }
        }

        fn record_data(&mut self, byte_count: usize, elapsed: Duration) {
            self.data_chunks += 1;
            self.data_bytes += byte_count as u64;
            self.process_time += elapsed;
            self.max_process_time = self.max_process_time.max(elapsed);
        }

        fn record_command(&mut self, elapsed: Duration) {
            self.command_count += 1;
            self.command_time += elapsed;
            self.max_command_time = self.max_command_time.max(elapsed);
        }

        fn record_responses(&mut self, count: usize, byte_count: usize, elapsed: Duration) {
            self.response_count += count as u64;
            self.response_bytes += byte_count as u64;
            self.response_send_time += elapsed;
            self.max_response_send_time = self.max_response_send_time.max(elapsed);
        }

        fn record_frame_build(&mut self, elapsed: Duration) {
            self.frame_build_count += 1;
            self.frame_build_time += elapsed;
            self.max_frame_build_time = self.max_frame_build_time.max(elapsed);
        }

        fn record_publish(&mut self, elapsed: Duration) {
            self.publish_attempts += 1;
            self.publish_time += elapsed;
            self.max_publish_time = self.max_publish_time.max(elapsed);
        }

        fn maybe_log(&mut self, workspace_id: &str) {
            let Some(started_at) = self.started_at else {
                *self = Self::new();
                return;
            };
            let elapsed = started_at.elapsed();
            if elapsed < PERF_LOG_INTERVAL {
                return;
            }
            if self.data_chunks > 0 || self.command_count > 0 || self.frame_build_count > 0 {
                log::debug!(
                    "SSH terminal perf window: workspace_id={workspace_id}, interval_ms={}, data_chunks={}, data_bytes={}, process_total_us={}, process_max_us={}, replies={}, reply_bytes={}, reply_send_total_us={}, reply_send_max_us={}, commands={}, command_total_us={}, command_max_us={}, frame_builds={}, frame_build_total_us={}, frame_build_max_us={}, publish_attempts={}, publish_attempt_total_us={}, publish_attempt_max_us={}",
                    elapsed.as_millis(),
                    self.data_chunks,
                    self.data_bytes,
                    self.process_time.as_micros(),
                    self.max_process_time.as_micros(),
                    self.response_count,
                    self.response_bytes,
                    self.response_send_time.as_micros(),
                    self.max_response_send_time.as_micros(),
                    self.command_count,
                    self.command_time.as_micros(),
                    self.max_command_time.as_micros(),
                    self.frame_build_count,
                    self.frame_build_time.as_micros(),
                    self.max_frame_build_time.as_micros(),
                    self.publish_attempts,
                    self.publish_time.as_micros(),
                    self.max_publish_time.as_micros(),
                );
            } else if !self.reported_idle {
                log::debug!(
                    "SSH terminal idle window: workspace_id={workspace_id}, interval_ms={}, reason=no_channel_data_or_terminal_commands",
                    elapsed.as_millis(),
                );
                self.reported_idle = true;
            }
            let reported_idle = self.reported_idle;
            *self = Self::new();
            self.reported_idle = reported_idle;
        }
    }

    pub(crate) struct TerminalSessionExit {
        pub(crate) commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        pub(crate) exit_message: Option<String>,
        pub(crate) remote_closed: bool,
        pub(crate) stop_reason: &'static str,
    }

    pub(crate) async fn run_connected_terminal_session(
        workspace_id: &str,
        shell: &dyn SshShell,
        mut commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) -> Result<TerminalSessionExit> {
        let mut refresh = tokio::time::interval(REFRESH_INTERVAL);
        refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        refresh.tick().await;

        let mut last_frame = model
            .read()
            .ok_or_else(|| anyhow::anyhow!("SSH workspace data unavailable"))?
            .data
            .frame
            .clone();
        let mut frame_dirty = false;
        let mut content_dirty = false;
        let mut exit_message = None;
        let mut remote_eof_deadline = None;
        let mut perf = TerminalPerfWindow::new();

        let stop_reason = loop {
            tokio::select! {
                biased;
                command = commands.recv() => {
                    if frame_dirty {
                        publish_model(
                            &model,
                            &mut last_frame,
                            content_dirty,
                            &mut perf,
                        );
                        frame_dirty = false;
                        content_dirty = false;
                    }
                    let command_started = Instant::now();
                    let keep_running = apply_command(
                        command,
                        shell,
                        &model,
                        &mut frame_dirty,
                        workspace_id,
                    ).await?;
                    perf.record_command(command_started.elapsed());
                    perf.maybe_log(workspace_id);
                    if !keep_running {
                        log::debug!(
                            "SSH terminal runtime stopping: workspace_id={workspace_id}, reason=application_command"
                        );
                        break ("application_command", false);
                    }
                }
                _ = async {
                    match remote_eof_deadline {
                        Some(deadline) => tokio::time::sleep_until(deadline).await,
                        None => std::future::pending::<()>().await,
                    }
                }, if remote_eof_deadline.is_some() => {
                    log::warn!(
                        "SSH terminal channel close timeout: workspace_id={workspace_id}, reason=remote_eof_without_close, grace_ms={}",
                        CHANNEL_CLOSE_GRACE_PERIOD.as_millis()
                    );
                    break ("remote_eof_timeout", true);
                }
                _ = refresh.tick() => {
                    if frame_dirty {
                        publish_model(
                            &model,
                            &mut last_frame,
                            content_dirty,
                            &mut perf,
                        );
                        frame_dirty = false;
                        content_dirty = false;
                    }
                    perf.maybe_log(workspace_id);
                }
                message = shell.wait_event() => match message {
                    Some(SshChannelEvent::Data(data)) => {
                        let process_started = Instant::now();
                        let responses = model.process(&data);
                        perf.record_data(data.len(), process_started.elapsed());
                        let response_count = responses.len();
                        let mut response_bytes = 0usize;
                        let response_started = Instant::now();
                        for response in responses {
                            response_bytes += response.len();
                            shell.send_data(response).await?;
                        }
                        if response_count > 0 {
                            perf.record_responses(
                                response_count,
                                response_bytes,
                                response_started.elapsed(),
                            );
                        }
                        frame_dirty = true;
                        content_dirty = true;
                        perf.maybe_log(workspace_id);
                    }
                    Some(SshChannelEvent::ExitStatus(exit_status)) => {
                        log::debug!(
                            "SSH remote shell exit status received: workspace_id={workspace_id}, exit_status={exit_status}"
                        );
                        exit_message = Some(format!(
                            "远程 Shell 已退出（状态码 {exit_status}）"
                        ));
                    }
                    Some(SshChannelEvent::ExitSignal { signal_name, core_dumped, error_message }) => {
                        log::warn!(
                            "SSH remote shell exit signal received: workspace_id={workspace_id}, signal={signal_name}, core_dumped={core_dumped}, error_message={error_message}"
                        );
                        exit_message = Some(format!(
                            "远程 Shell 被信号终止（信号 {signal_name}）"
                        ));
                    }
                    Some(SshChannelEvent::Eof) => {
                        if remote_eof_deadline.is_none() {
                            remote_eof_deadline = Some(
                                tokio::time::Instant::now() + CHANNEL_CLOSE_GRACE_PERIOD,
                            );
                            log::warn!(
                                "SSH terminal channel EOF received: workspace_id={workspace_id}, channel_state=remote_output_closed, awaiting=exit_status_or_close, grace_ms={}",
                                CHANNEL_CLOSE_GRACE_PERIOD.as_millis()
                            );
                        } else {
                            log::debug!(
                                "SSH terminal channel EOF repeated: workspace_id={workspace_id}"
                            );
                        }
                    }
                    Some(SshChannelEvent::Close) => {
                        log::warn!(
                            "SSH terminal channel closed by peer: workspace_id={workspace_id}, reason={}, eof_received={}",
                            if remote_eof_deadline.is_some() { "remote_close_after_eof" } else { "remote_close" },
                            remote_eof_deadline.is_some()
                        );
                        break (
                            if remote_eof_deadline.is_some() {
                                "remote_close_after_eof"
                            } else {
                                "remote_close"
                            },
                            true,
                        );
                    }
                    None => {
                        log::warn!(
                            "SSH terminal channel reader ended: workspace_id={workspace_id}, reason={}, eof_received={}",
                            if remote_eof_deadline.is_some() { "reader_closed_after_eof" } else { "reader_closed" },
                            remote_eof_deadline.is_some()
                        );
                        break (
                            if remote_eof_deadline.is_some() {
                                "reader_closed_after_eof"
                            } else {
                                "reader_closed"
                            },
                            true,
                        );
                    }
                    Some(SshChannelEvent::Other) => {}
                }
            }
        };

        if frame_dirty {
            publish_model(&model, &mut last_frame, content_dirty, &mut perf);
        }
        perf.maybe_log(workspace_id);
        log::debug!(
            "SSH terminal runtime ended: workspace_id={workspace_id}, reason={}, mcp_snapshot_version={}, gui_snapshot_version={}",
            stop_reason.0,
            model.mcp_snapshot_version(),
            model.gui_snapshot_version()
        );
        Ok(TerminalSessionExit {
            commands,
            exit_message,
            remote_closed: stop_reason.1,
            stop_reason: stop_reason.0,
        })
    }

    pub(crate) async fn run_closed_terminal_session(
        workspace_id: &str,
        mut commands: mpsc::UnboundedReceiver<TerminalSessionCommand>,
        model: Arc<TerminalModel>,
    ) -> Result<()> {
        let mut last_frame = model
            .read()
            .ok_or_else(|| anyhow::anyhow!("SSH workspace data unavailable"))?
            .data
            .frame
            .clone();
        let mut perf = TerminalPerfWindow::new();
        let stop_reason = loop {
            let Some(command) = commands.recv().await else {
                break "command_channel_closed";
            };
            let mut frame_dirty = false;
            match command {
                TerminalSessionCommand::Input(data) => {
                    log::debug!(
                        "SSH terminal input ignored after disconnect: workspace_id={workspace_id}, bytes={}",
                        data.len()
                    );
                }
                TerminalSessionCommand::Resize { columns, rows } => {
                    model.resize(columns, rows);
                    frame_dirty = true;
                }
                TerminalSessionCommand::Scroll { lines } => {
                    model.scroll(lines);
                    frame_dirty = true;
                }
                TerminalSessionCommand::ScrollTo { offset } => {
                    model.scroll_to(offset);
                    frame_dirty = true;
                }
                TerminalSessionCommand::Disconnect => {
                    log::info!(
                        "SSH terminal history runtime stopping: workspace_id={workspace_id}, reason=application_close"
                    );
                    break "application_close";
                }
            }
            if frame_dirty {
                publish_model(&model, &mut last_frame, false, &mut perf);
                perf.maybe_log(workspace_id);
            }
        };
        log::debug!(
            "SSH terminal history runtime ended: workspace_id={workspace_id}, reason={stop_reason}, mcp_snapshot_version={}, gui_snapshot_version={}",
            model.mcp_snapshot_version(),
            model.gui_snapshot_version()
        );
        Ok(())
    }

    async fn apply_command(
        command: Option<TerminalSessionCommand>,
        shell: &dyn SshShell,
        model: &TerminalModel,
        frame_dirty: &mut bool,
        workspace_id: &str,
    ) -> Result<bool> {
        match command {
            Some(TerminalSessionCommand::Input(data)) => {
                if !data.is_empty() {
                    let input_bytes = data.len();
                    shell.send_data(data).await?;
                    log::debug!(
                        "SSH terminal input written: workspace_id={workspace_id}, bytes={}",
                        input_bytes
                    );
                }
            }
            Some(TerminalSessionCommand::Resize { columns, rows }) => {
                model.resize(columns, rows);
                shell.resize(columns, rows).await?;
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::Scroll { lines }) => {
                model.scroll(lines);
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::ScrollTo { offset }) => {
                model.scroll_to(offset);
                *frame_dirty = true;
            }
            Some(TerminalSessionCommand::Disconnect) => {
                log::info!(
                    "SSH terminal close requested: workspace_id={workspace_id}, reason=application_close, action=channel_eof_and_close"
                );
                close_channel(workspace_id, shell).await;
                return Ok(false);
            }
            None => {
                log::debug!(
                    "SSH terminal command channel closed: workspace_id={workspace_id}, action=channel_eof_and_close"
                );
                close_channel(workspace_id, shell).await;
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn close_channel(workspace_id: &str, shell: &dyn SshShell) {
        if let Err(error) = shell.close_channel().await {
            log::debug!(
                "SSH terminal channel close failed: workspace_id={workspace_id}, error={error:#}"
            );
        }
    }

    fn publish_model(
        model: &TerminalModel,
        last_frame: &mut Arc<TerminalFrame>,
        content_changed: bool,
        perf: &mut TerminalPerfWindow,
    ) {
        let Some(snapshot) = model.read() else {
            return;
        };
        let frame_started = Instant::now();
        let Some(frame) = model.frame_reusing(Some(last_frame.as_ref())) else {
            return;
        };
        perf.record_frame_build(frame_started.elapsed());
        let next_frame = Arc::new(frame);
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
            status: snapshot.data.status.clone(),
            message: snapshot.data.message.clone(),
        };
        let publish_started = Instant::now();
        if content_changed {
            model.replace(data);
        } else {
            model.replace_view(data);
        }
        perf.record_publish(publish_started.elapsed());
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

pub(crate) use core::run_ssh_session;
