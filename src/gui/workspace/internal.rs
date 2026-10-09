use gpui_kit::Context;

use super::Workspace;

impl Workspace {
    pub(super) fn select_workspace(&mut self, workspace_id: Option<&str>, cx: &mut Context<Self>) {
        let protocol = self.protocol_for_workspace(workspace_id, cx);
        if self.active_protocol != protocol {
            self.active_protocol = protocol;
            cx.notify();
        }
    }
}
