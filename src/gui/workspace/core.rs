use gpui::Context;

use crate::domain::session::Protocol;

use super::{Workspace, session::terminal_statuses};

impl Workspace {
    pub(super) fn protocol_for_workspace(
        &self,
        workspace_id: Option<&str>,
        cx: &Context<Self>,
    ) -> Option<Protocol> {
        let workspace_id = workspace_id?;
        self.workspace
            .read(cx)
            .sessions()
            .iter()
            .find(|opened| opened.id == workspace_id)
            .map(|opened| opened.profile.protocol)
    }

    pub(super) fn refresh_session_statuses(&self, cx: &mut Context<Self>) {
        let statuses = terminal_statuses(self.workspace.read(cx).sessions(), |id| {
            self.terminal
                .read(cx)
                .model(id)
                .map(|model| model.read().status.clone())
                .or_else(|| self.sftp.read(cx).connection_status(id))
        });
        self.workspace
            .update(cx, |workspace, cx| workspace.update_statuses(statuses, cx));
    }
}
