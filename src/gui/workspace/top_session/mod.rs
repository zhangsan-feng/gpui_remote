mod core;
mod external;
mod internal;
mod ui;

use std::collections::HashMap;

use gpui::*;

use crate::component::draggable_list::DraggableList;
use crate::domain::session::SessionProfile;
use crate::domain::terminal::TerminalStatus;

pub(super) use external::terminal_statuses;

#[derive(Clone, Debug)]
pub(super) struct OpenedWorkspaceSession {
    pub(super) id: String,
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
        let this = Self {
            sessions: Vec::new(),
            tabs: cx.new(|cx| ui::new_workspace_tabs(cx)),
            selected_id: None,
            statuses: HashMap::new(),
        };
        this.start_subscribe(cx);
        this
    }
}

impl Render for WorkspaceSession {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.tabs.clone())
    }
}
