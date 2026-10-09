mod core;
mod external;
mod internal;
mod ui;

use std::collections::HashMap;

use gpui_kit::*;

use crate::component::draggable_list::DraggableList;
use crate::data_context::WorkspaceSummary;
use crate::domain::terminal::TerminalStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OpenedWorkspaceSession {
    pub(super) id: String,
    pub(super) profile: WorkspaceSummary,
}

pub(super) struct WorkspaceSession {
    sessions: Vec<OpenedWorkspaceSession>,
    tabs: Entity<DraggableList>,
    selected_id: Option<String>,
    statuses: HashMap<String, TerminalStatus>,
}

impl WorkspaceSession {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            sessions: Vec::new(),
            tabs: cx.new(|cx| ui::new_workspace_tabs(cx)),
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
