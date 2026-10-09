use std::sync::Arc;

use crate::data_context::DATA_CONTEXT;
use tokio::sync::mpsc;

use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::{TerminalData, TerminalFrame, TerminalSessionCommand, TerminalStatus},
};

use super::run_ssh_session;

pub(crate) struct TerminalModel {
    workspace_id: String,
}

pub(crate) struct TerminalRuntime {
    pub(crate) model: Arc<TerminalModel>,
    pub(crate) commands: mpsc::UnboundedSender<TerminalSessionCommand>,
    pub(crate) task: Option<tokio::task::JoinHandle<()>>,
}

impl TerminalModel {
    pub(crate) fn new(workspace_id: String) -> Self {
        Self { workspace_id }
    }

    pub(crate) fn read(&self) -> Option<crate::data_context::TerminalReadSnapshot> {
        DATA_CONTEXT.terminal_snapshot(&self.workspace_id)
    }

    pub(crate) fn replace(&self, data: TerminalData) -> u64 {
        DATA_CONTEXT.publish_terminal(&self.workspace_id, data, true);
        self.mcp_snapshot_version()
    }

    pub(crate) fn replace_view(&self, data: TerminalData) {
        DATA_CONTEXT.publish_terminal(&self.workspace_id, data, false);
    }

    pub(crate) fn mcp_snapshot_version(&self) -> u64 {
        self.read()
            .map_or(0, |snapshot| snapshot.mcp_snapshot_version)
    }

    pub(crate) fn gui_snapshot_version(&self) -> u64 {
        self.read()
            .map_or(0, |snapshot| snapshot.gui_snapshot_version)
    }

    pub(crate) fn set_status(&self, status: TerminalStatus, message: Option<String>) {
        DATA_CONTEXT.update_terminal_status(&self.workspace_id, status, message);
    }

    pub(crate) fn initialize_buffer(&self) -> bool {
        DATA_CONTEXT.initialize_terminal_buffer(&self.workspace_id)
    }

    pub(crate) fn process(&self, bytes: &[u8]) -> Vec<Vec<u8>> {
        DATA_CONTEXT
            .with_terminal_buffer(&self.workspace_id, |buffer| {
                buffer.process(bytes);
                buffer.drain_pty_writes()
            })
            .unwrap_or_default()
    }

    pub(crate) fn resize(&self, columns: u32, rows: u32) -> bool {
        DATA_CONTEXT
            .with_terminal_buffer(&self.workspace_id, |buffer| buffer.resize(columns, rows))
            .is_some()
    }

    pub(crate) fn scroll(&self, lines: i32) -> bool {
        DATA_CONTEXT
            .with_terminal_buffer(&self.workspace_id, |buffer| buffer.scroll(lines))
            .is_some()
    }

    pub(crate) fn scroll_to(&self, offset: usize) -> bool {
        DATA_CONTEXT
            .with_terminal_buffer(&self.workspace_id, |buffer| buffer.scroll_to(offset))
            .is_some()
    }

    pub(crate) fn frame_reusing(&self, previous: Option<&TerminalFrame>) -> Option<TerminalFrame> {
        DATA_CONTEXT
            .with_terminal_buffer(&self.workspace_id, |buffer| buffer.frame_reusing(previous))
    }
}

pub(crate) fn new_runtime(workspace_id: String, profile: SessionProfile) -> TerminalRuntime {
    let model = Arc::new(TerminalModel::new(workspace_id.clone()));
    let (commands, command_rx) = mpsc::unbounded_channel();
    let task = if supports_terminal_protocol(&profile.protocol) {
        if !model.initialize_buffer() {
            log::warn!("SSH terminal buffer initialization skipped: workspace_id={workspace_id}");
        }
        Some(tokio::spawn(run_ssh_session(
            workspace_id,
            profile,
            command_rx,
            model.clone(),
        )))
    } else {
        model.set_status(
            TerminalStatus::Failed,
            Some(format!("暂不支持 {} 终端连接", profile.protocol)),
        );
        None
    };
    TerminalRuntime {
        model,
        commands,
        task,
    }
}

pub(crate) fn disconnect(runtime: TerminalRuntime) {
    if runtime
        .commands
        .send(TerminalSessionCommand::Disconnect)
        .is_ok()
    {
        log::debug!(
            "SSH runtime disconnect requested: mode=graceful, action=send_disconnect_command"
        );
    } else {
        log::debug!("SSH runtime disconnect requested: mode=forced, reason=command_channel_closed");
        if let Some(task) = runtime.task {
            task.abort();
            log::debug!("SSH runtime task aborted after command channel closed");
        }
    }
}

pub(crate) fn supports_terminal_protocol(protocol: &Protocol) -> bool {
    matches!(protocol, Protocol::Ssh)
}
