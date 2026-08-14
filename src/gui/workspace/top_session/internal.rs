use crate::domain::terminal::TerminalStatus;
use gpui::{Context, ElementId};

use super::{WorkspaceSession, ui::workspace_tab};

impl WorkspaceSession {
    pub(super) fn rebuild_tabs(&mut self, cx: &mut Context<Self>) {
        let sessions = self.sessions.clone();
        let statuses = self.statuses.clone();
        let selected_id = self.selected_id.clone();
        let workspace = cx.entity();
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.clear(tabs_cx);
            for opened_session in sessions {
                let workspace = workspace.clone();
                let status = statuses
                    .get(&opened_session.id)
                    .cloned()
                    .unwrap_or(TerminalStatus::Connecting);
                tabs.child_with_context(opened_session.id.clone(), move |cx| {
                    workspace_tab(
                        opened_session.id.clone(),
                        opened_session.profile.clone(),
                        status.clone(),
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

    pub(super) fn select_tab(&self, id: &str, cx: &mut Context<Self>) {
        let id = ElementId::from(id.to_owned());
        self.tabs.update(cx, |tabs, tabs_cx| {
            tabs.set_selected_id(&id, tabs_cx);
        });
    }
}
