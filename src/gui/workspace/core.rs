use gpui_kit::Context;

use crate::domain::session::Protocol;

use super::Workspace;

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
}
