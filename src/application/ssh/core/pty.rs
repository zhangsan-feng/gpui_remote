use std::sync::{
    Arc, RwLock, RwLockReadGuard,
    atomic::{AtomicU64, Ordering},
};

use tokio::sync::{Notify, mpsc};

use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::{TerminalData, TerminalFrame, TerminalSessionCommand, TerminalStatus},
};

use super::run_ssh_session;

pub(crate) struct TerminalModel {
    data: RwLock<TerminalData>,
    mcp_snapshot_version: AtomicU64,
    gui_snapshot_version: AtomicU64,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

pub(crate) struct TerminalRuntime {
    pub(crate) model: Arc<TerminalModel>,
    pub(crate) commands: mpsc::UnboundedSender<TerminalSessionCommand>,
    pub(crate) task: Option<tokio::task::JoinHandle<()>>,
}

impl TerminalModel {
    pub(crate) fn new(
        data: TerminalData,
        updates: Arc<Notify>,
        status_updates: Arc<Notify>,
    ) -> Self {
        Self {
            data: RwLock::new(data),
            mcp_snapshot_version: AtomicU64::new(0),
            gui_snapshot_version: AtomicU64::new(0),
            updates,
            status_updates,
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, TerminalData> {
        self.data
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn replace(&self, data: TerminalData) -> u64 {
        *self
            .data
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = data;
        let mcp_snapshot_version = self.mcp_snapshot_version.fetch_add(1, Ordering::Release) + 1;
        self.gui_snapshot_version.fetch_add(1, Ordering::Release);
        self.updates.notify_waiters();
        mcp_snapshot_version
    }

    pub(crate) fn replace_view(&self, data: TerminalData) {
        *self
            .data
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = data;
        self.gui_snapshot_version.fetch_add(1, Ordering::Release);
        self.updates.notify_waiters();
    }

    pub(crate) fn mcp_snapshot_version(&self) -> u64 {
        self.mcp_snapshot_version.load(Ordering::Acquire)
    }

    pub(crate) fn gui_snapshot_version(&self) -> u64 {
        self.gui_snapshot_version.load(Ordering::Acquire)
    }

    pub(crate) fn set_status(&self, status: TerminalStatus, message: Option<String>) {
        {
            let mut data = self
                .data
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            data.status = status;
            data.message = message;
        }
        self.gui_snapshot_version.fetch_add(1, Ordering::Release);
        self.updates.notify_waiters();
        self.status_updates.notify_waiters();
    }
}

pub(crate) fn new_runtime(
    workspace_id: String,
    profile: SessionProfile,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
) -> TerminalRuntime {
    let model = Arc::new(TerminalModel::new(
        TerminalData {
            frame: Arc::new(TerminalFrame::default()),
            status: TerminalStatus::Connecting,
            message: Some("正在建立 SSH 连接…".into()),
        },
        updates,
        status_updates,
    ));
    let (commands, command_rx) = mpsc::unbounded_channel();
    let task = if supports_terminal_protocol(&profile.protocol) {
        Some(tokio::spawn(run_ssh_session(
            workspace_id,
            profile,
            commands.clone(),
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
