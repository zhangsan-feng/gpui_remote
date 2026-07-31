use gpui::Context;

use crate::global_state::{GlobalEvent, read_global_state};

use super::Workspace;

impl Workspace {
    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            let GlobalEvent::SelectWorkspaceSession(workspace_id) = event else {
                return;
            };
            this.select_workspace(workspace_id.as_deref(), cx);
        })
        .detach();
    }
}
