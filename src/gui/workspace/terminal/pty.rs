use std::sync::{
    Arc, RwLock, RwLockReadGuard,
    atomic::{AtomicU64, Ordering},
};

use tokio::sync::{Notify, mpsc};

use crate::domain::{
    session::SessionProfile,
    terminal::{TerminalData, TerminalFrame, TerminalSessionCommand, TerminalStatus},
};

use super::{TerminalView, ssh::run_ssh_session};

pub(in crate::gui::workspace) struct TerminalModel {
    data: RwLock<TerminalData>,
    revision: AtomicU64,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

pub(super) struct TerminalRuntime {
    model: Arc<TerminalModel>,
    commands: mpsc::UnboundedSender<TerminalSessionCommand>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl TerminalModel {
    pub(crate) fn new(
        data: TerminalData,
        updates: Arc<Notify>,
        status_updates: Arc<Notify>,
    ) -> Self {
        Self {
            data: RwLock::new(data),
            revision: AtomicU64::new(0),
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
        let revision = self.revision.fetch_add(1, Ordering::Release) + 1;
        self.updates.notify_waiters();
        revision
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
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
        self.revision.fetch_add(1, Ordering::Release);
        self.updates.notify_waiters();
        self.status_updates.notify_waiters();
    }
}

impl TerminalView {
    pub(in crate::gui::workspace) fn model(
        &self,
        workspace_id: &str,
    ) -> Option<Arc<TerminalModel>> {
        Some(self.terminals.get(workspace_id)?.model.clone())
    }

    pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<Notify> {
        self.status_updates.clone()
    }

    pub(super) fn connect(&mut self, workspace_id: String, profile: SessionProfile) {
        let model = Arc::new(TerminalModel::new(
            TerminalData {
                frame: Arc::new(TerminalFrame::default()),
                status: TerminalStatus::Connecting,
                message: Some("正在建立 SSH 连接…".into()),
            },
            self.updates.clone(),
            self.status_updates.clone(),
        ));
        let (commands, command_rx) = mpsc::unbounded_channel();

        if !supports_terminal_protocol(&profile.protocol) {
            model.set_status(
                TerminalStatus::Failed,
                Some(format!("暂不支持 {} 终端连接", profile.protocol)),
            );
            self.terminals.insert(
                workspace_id,
                TerminalRuntime {
                    model,
                    commands,
                    task: None,
                },
            );
            return;
        }

        let task = tokio::spawn(run_ssh_session(
            profile,
            commands.clone(),
            command_rx,
            model.clone(),
        ));
        self.terminals.insert(
            workspace_id,
            TerminalRuntime {
                model,
                commands,
                task: Some(task),
            },
        );
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        if let Some(terminal) = self.terminals.remove(workspace_id) {
            disconnect(terminal);
        }
    }

    pub(in crate::gui::workspace) fn command_sender(
        &self,
        workspace_id: &str,
    ) -> Option<mpsc::UnboundedSender<TerminalSessionCommand>> {
        Some(self.terminals.get(workspace_id)?.commands.clone())
    }

    pub(in crate::gui::workspace) fn send_input(&self, workspace_id: &str, input: Vec<u8>) {
        if let Some(terminal) = self.terminals.get(workspace_id) {
            let _ = terminal.commands.send(TerminalSessionCommand::Input(input));
        }
    }

    pub(super) fn resize(&self, workspace_id: &str, columns: u32, rows: u32) {
        if let Some(terminal) = self.terminals.get(workspace_id) {
            let _ = terminal
                .commands
                .send(TerminalSessionCommand::Resize { columns, rows });
        }
    }
}

impl Drop for TerminalView {
    fn drop(&mut self) {
        for (_, terminal) in self.terminals.drain() {
            disconnect(terminal);
        }
    }
}

fn disconnect(terminal: TerminalRuntime) {
    let _ = terminal.commands.send(TerminalSessionCommand::Disconnect);
    if let Some(task) = terminal.task {
        task.abort();
    }
}

fn supports_terminal_protocol(protocol: &str) -> bool {
    protocol.eq_ignore_ascii_case("SSH")
}
