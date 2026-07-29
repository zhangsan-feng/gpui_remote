mod core;
mod ui;

use std::collections::HashMap;

use gpui::*;

use crate::component::draggable_list::DraggableList;
use crate::domain::session::{SessionProfile, WorkspaceSession as WorkspaceSessionConnection};
use crate::domain::terminal::TerminalStatus;
use crate::global_state::{read_global_state, GlobalEvent, GlobalState};

pub(super) use core::terminal_statuses;

#[derive(Clone, Debug)]
pub(super) struct OpenedWorkspaceSession {
    pub(super) session: WorkspaceSessionConnection,
    pub(super) profile: SessionProfile,
}

pub(super) struct WorkspaceSession {
    sessions: Vec<OpenedWorkspaceSession>,
    tabs: Entity<DraggableList>,
    selected_id: Option<String>,
    statuses: HashMap<String, TerminalStatus>,

}

impl WorkspaceSession {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            let GlobalEvent::CreateActiveSession(profile) = event else {
                return;
            };
            this.open(profile.clone(), cx);
        })
        .detach();

        Self {
            sessions: Vec::new(),
            tabs: cx.new(|_| ui::new_workspace_tabs()),
            selected_id: None,
            statuses: HashMap::new(),
        }
    }
}

impl Render for WorkspaceSession {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.tabs.clone())
    }
}
