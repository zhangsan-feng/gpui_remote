use gpui::Context;

use crate::domain::session::{SessionProfile, WorkspaceSession as WorkspaceSessionConnection};

use super::{OpenedWorkspaceSession, WorkspaceSession, WorkspaceSessionEvent};

impl WorkspaceSession {
    pub fn sessions(&self) -> &[OpenedWorkspaceSession] {
        &self.sessions
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.active_session_id.as_deref() == Some(id)
            || !self.sessions.iter().any(|item| item.session.id == id)
        {
            return;
        }
        self.active_session_id = Some(id.to_owned());
        cx.emit(WorkspaceSessionEvent::Changed);
    }

    pub fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(index) = self.sessions.iter().position(|item| item.session.id == id) else {
            return;
        };
        self.sessions.remove(index);
        if self.active_session_id.as_deref() == Some(id) {
            self.active_session_id = next_active_session(&self.sessions, index);
        }
        cx.emit(WorkspaceSessionEvent::Closed {
            workspace_ids: vec![id.to_owned()],
        });
    }

    pub(super) fn open(&mut self, profile: SessionProfile, cx: &mut Context<Self>) {
        let session = WorkspaceSessionConnection::new(profile.id.clone());
        let workspace_id = session.id.clone();
        self.sessions.push(OpenedWorkspaceSession {
            session,
            profile: profile.clone(),
        });
        self.active_session_id = Some(workspace_id.clone());
        cx.emit(WorkspaceSessionEvent::Opened {
            workspace_id,
            profile,
        });
    }

    pub(super) fn close_profile(&mut self, profile_id: &str, cx: &mut Context<Self>) {
        let removed_ids = self
            .sessions
            .iter()
            .filter(|item| item.session.profile_id == profile_id)
            .map(|item| item.session.id.clone())
            .collect::<Vec<_>>();
        if removed_ids.is_empty() {
            return;
        }
        self.sessions
            .retain(|item| item.session.profile_id != profile_id);
        if self
            .active_session_id
            .as_ref()
            .is_some_and(|active_id| removed_ids.contains(active_id))
        {
            self.active_session_id = self.sessions.first().map(|item| item.session.id.clone());
        }
        cx.emit(WorkspaceSessionEvent::Closed {
            workspace_ids: removed_ids,
        });
    }
}

fn next_active_session(
    sessions: &[OpenedWorkspaceSession],
    removed_index: usize,
) -> Option<String> {
    sessions
        .get(removed_index.min(sessions.len().saturating_sub(1)))
        .map(|item| item.session.id.clone())
}
