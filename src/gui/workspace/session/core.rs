use std::collections::HashMap;

use gpui::{Context, ElementId};

use super::{OpenedWorkspaceSession, WorkspaceSession, ui::workspace_tab};
use crate::global_state::read_global_state;
use crate::{
    domain::{
        session::{SessionProfile, WorkspaceSession as WorkspaceSessionConnection},
        terminal::TerminalStatus,
    },
    global_state::GlobalEvent,
};

impl WorkspaceSession {
    pub fn sessions(&self) -> &[OpenedWorkspaceSession] {
        &self.sessions
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected_id.as_deref()
    }

    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected_id.as_deref() == Some(id)
            || !self.sessions.iter().any(|item| item.session.id == id)
        {
            return;
        }
        self.selected_id = Some(id.to_owned());
        self.select_tab(id, cx);
        cx.notify();
    }

    pub fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sessions.iter().position(|item| item.session.id == id) else {
            return;
        };
        self.sessions.remove(index);
        self.statuses.remove(id);
        if self.selected_id.as_deref() == Some(id) {
            self.selected_id = next_selected_session(&self.sessions, index);
        }
        self.rebuild_tabs(cx);

        let workspace_id = id.to_owned();
        let global_state = read_global_state(cx);
        global_state.update(cx, |_, cx| {
            cx.emit(GlobalEvent::CloseActiveSession(workspace_id));
        });
        cx.notify();
    }

    pub(in crate::gui::workspace) fn open(
        &mut self,
        profile: SessionProfile,
        cx: &mut Context<Self>,
    ) -> String {
        let session = WorkspaceSessionConnection::new(profile.id.clone());
        let workspace_id = session.id.clone();
        let opened_workspace_id = workspace_id.clone();
        let terminal_profile = profile.clone();
        self.sessions
            .push(OpenedWorkspaceSession { session, profile });
        self.selected_id = Some(workspace_id.clone());
        self.rebuild_tabs(cx);

        let global_state = read_global_state(cx);
        cx.defer(move |cx| {
            global_state.update(cx, |_, cx| {
                cx.emit(GlobalEvent::CreateActiveSessionTerminal {
                    workspace_id,
                    profile: terminal_profile,
                });
            });
        });
        cx.notify();
        opened_workspace_id
    }

    pub(in crate::gui::workspace) fn update_statuses(
        &mut self,
        statuses: HashMap<String, TerminalStatus>,
        cx: &mut Context<Self>,
    ) {
        if self.statuses != statuses {
            self.statuses = statuses;
            self.rebuild_tabs(cx);
        }
    }

    fn rebuild_tabs(&mut self, cx: &mut Context<Self>) {
        let sessions = self.sessions.clone();
        let statuses = self.statuses.clone();
        let selected_id = self.selected_id.clone();
        let workspace = cx.entity();
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.clear(tabs_cx);
            for opened_session in sessions {
                let workspace = workspace.clone();
                let status = statuses
                    .get(&opened_session.session.id)
                    .cloned()
                    .unwrap_or(TerminalStatus::Connecting);
                tabs.child(opened_session.session.id.clone(), move || {
                    workspace_tab(
                        opened_session.session.clone(),
                        opened_session.profile.clone(),
                        status.clone(),
                        workspace.clone(),
                    )
                });
            }
            if let Some(selected_id) = selected_id {
                let selected_id = ElementId::from(selected_id);
                tabs.set_selected_id(&selected_id, tabs_cx);
            }
        });
    }

    fn select_tab(&self, id: &str, cx: &mut Context<Self>) {
        let id = ElementId::from(id.to_owned());
        self.tabs.update(cx, |tabs, tabs_cx| {
            tabs.set_selected_id(&id, tabs_cx);
        });
    }
}

pub(in crate::gui::workspace) fn terminal_statuses(
    sessions: &[OpenedWorkspaceSession],
    terminal_status: impl Fn(&str) -> Option<TerminalStatus>,
) -> HashMap<String, TerminalStatus> {
    sessions
        .iter()
        .map(|opened_session| {
            let id = opened_session.session.id.clone();
            let status = terminal_status(&id).unwrap_or(TerminalStatus::Connecting);
            (id, status)
        })
        .collect()
}

fn next_selected_session(
    sessions: &[OpenedWorkspaceSession],
    removed_index: usize,
) -> Option<String> {
    sessions
        .get(removed_index.min(sessions.len().saturating_sub(1)))
        .map(|item| item.session.id.clone())
}
