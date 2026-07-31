use gpui::Context;

use super::Workspace;

impl Workspace {
    pub(super) fn start_status_watchers(&self, cx: &mut Context<Self>) {
        let terminal_updates = self.terminal.read(cx).status_updates();
        cx.spawn(async move |this, cx| {
            loop {
                terminal_updates.notified().await;
                if this
                    .update(cx, |this, cx| this.refresh_session_statuses(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let sftp_updates = self.sftp.read(cx).status_updates();
        cx.spawn(async move |this, cx| {
            loop {
                sftp_updates.notified().await;
                if this
                    .update(cx, |this, cx| this.refresh_session_statuses(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn select_workspace(&mut self, workspace_id: Option<&str>, cx: &mut Context<Self>) {
        let protocol = self.protocol_for_workspace(workspace_id, cx);
        if self.active_protocol != protocol {
            self.active_protocol = protocol;
            cx.notify();
        }
    }
}
