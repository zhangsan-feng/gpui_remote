mod core;
mod ui;

use gpui::{Context, Entity, EventEmitter, Subscription};

use crate::domain::session::{SessionProfile, WorkspaceSession as WorkspaceSessionConnection};
use crate::global_state::{GlobalEvent, GlobalState};

pub(super) use ui::workspace_tab;

#[derive(Clone, Debug)]
pub(super) struct OpenedWorkspaceSession {
    pub(super) session: WorkspaceSessionConnection,
    pub(super) profile: SessionProfile,
}

#[derive(Clone, Debug)]
pub(super) enum WorkspaceSessionEvent {
    Changed,
    Opened {
        workspace_id: String,
        profile: SessionProfile,
    },
    Closed {
        workspace_ids: Vec<String>,
    },
}

pub(super) struct WorkspaceSession {
    sessions: Vec<OpenedWorkspaceSession>,
    active_session_id: Option<String>,
    _global_subscription: Subscription,
}

impl WorkspaceSession {
    pub fn new(global_state: Entity<GlobalState>, cx: &mut Context<Self>) -> Self {
        let global_subscription = cx.subscribe(&global_state, |this, _, event, cx| match event {
            GlobalEvent::CreateSession | GlobalEvent::UpdateSession => {}
            GlobalEvent::CreateActiveSession(profile) => this.open(profile.clone(), cx),
            GlobalEvent::SessionProfileDeleted(profile_id) => this.close_profile(profile_id, cx),
        });

        Self {
            sessions: Vec::new(),
            active_session_id: None,
            _global_subscription: global_subscription,
        }
    }
}

impl EventEmitter<WorkspaceSessionEvent> for WorkspaceSession {}
