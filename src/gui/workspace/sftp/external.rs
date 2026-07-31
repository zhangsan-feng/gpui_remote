use std::sync::Arc;

use gpui::*;
use tokio::sync::Notify;

use crate::{
    domain::{session::Protocol, terminal::TerminalStatus},
    global_state::{GlobalEvent, read_global_state},
};

use super::{SftpStatus, SftpView};

impl SftpView {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            match event {
                GlobalEvent::OpenWorkspaceSession(workspace_id, profile)
                    if profile.protocol == Protocol::Sftp =>
                {
                    this.connect(workspace_id.clone(), profile.clone());
                }
                GlobalEvent::SelectWorkspaceSession(workspace_id) => {
                    if this.selected_workspace_id == *workspace_id {
                        return;
                    }
                    this.selected_workspace_id = workspace_id.clone();
                    this.remote_list_state.reset_with_uniform_height(0, px(38.));
                }
                GlobalEvent::CloseWorkspaceSession { workspace_id } => {
                    this.close(workspace_id);
                }
                _ => return,
            }
            cx.notify();
        })
        .detach();
    }

    pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<Notify> {
        self.status_updates.clone()
    }

    pub(in crate::gui::workspace) fn connection_status(
        &self,
        workspace_id: &str,
    ) -> Option<TerminalStatus> {
        let status = self.runtimes.get(workspace_id)?.model.snapshot().status;
        Some(match status {
            SftpStatus::Connecting => TerminalStatus::Connecting,
            SftpStatus::Connected => TerminalStatus::Connected,
            SftpStatus::Disconnected => TerminalStatus::Disconnected,
            SftpStatus::Failed => TerminalStatus::Failed,
        })
    }
}
