use crate::domain::terminal::TerminalStatus;
use gpui_kit::{Context, ElementId};

use super::{WorkspaceSession, ui::workspace_tab};

impl WorkspaceSession {
    pub(super) fn refresh_tab(&mut self, workspace_id: &str, cx: &mut Context<Self>) {
        let Some(session) = self
            .sessions
            .iter()
            .find(|session| session.id == workspace_id)
            .cloned()
        else {
            return;
        };
        let status = session.profile.status.clone();
        let selected = self.selected_id.as_deref() == Some(workspace_id);
        let workspace = cx.entity();
        let id = ElementId::from(workspace_id.to_owned());
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.update_child_with_context(
                &id,
                move |cx| {
                    workspace_tab(
                        session.id.clone(),
                        session.profile.clone(),
                        status.clone(),
                        selected,
                        workspace.clone(),
                        cx,
                    )
                },
                tabs_cx,
            );
        });
    }

    pub(super) fn rebuild_tabs(&mut self, cx: &mut Context<Self>) {
        let sessions = self.sessions.clone();
        let statuses = self.statuses.clone();
        let selected_id = self.selected_id.clone();
        let workspace = cx.entity();
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.clear(tabs_cx);
            for opened_session in sessions {
                let workspace = workspace.clone();
                let selected = selected_id.as_deref() == Some(opened_session.id.as_str());
                let status = statuses
                    .get(&opened_session.id)
                    .cloned()
                    .unwrap_or(TerminalStatus::Connecting);
                tabs.child_with_context(opened_session.id.clone(), move |cx| {
                    workspace_tab(
                        opened_session.id.clone(),
                        opened_session.profile.clone(),
                        status.clone(),
                        selected,
                        workspace.clone(),
                        cx,
                    )
                });
            }
            if let Some(selected_id) = selected_id {
                let selected_id = ElementId::from(selected_id);
                tabs.set_selected_id(&selected_id, tabs_cx);
            }
        });
    }
}
