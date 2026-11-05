mod core;
mod external;
mod ui;

pub(super) use ui::workspace_tab_bar_background;

use std::collections::HashMap;

use gpui_kit::*;

use crate::data_context::WorkspaceSummary;
use crate::domain::terminal::TerminalStatus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct OpenedWorkspaceSession {
    pub(super) id: String,
    pub(super) profile: WorkspaceSummary,
}

pub(super) struct WorkspaceSession {
    gui_scope: EntityId,
    sessions: Vec<OpenedWorkspaceSession>,
    tabs_scroll: ScrollHandle,
    selected_id: Option<String>,
    drop_target: Option<(String, bool)>,
    statuses: HashMap<String, TerminalStatus>,
}

impl WorkspaceSession {
    pub fn new(gui_scope: EntityId, _cx: &mut Context<Self>) -> Self {
        Self {
            gui_scope,
            sessions: Vec::new(),
            tabs_scroll: ScrollHandle::new(),
            selected_id: None,
            drop_target: None,
            statuses: HashMap::new(),
        }
    }
}

impl Render for WorkspaceSession {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_tabs(cx)
    }
}

#[derive(Clone)]
struct DraggedWorkspaceTab {
    owner: EntityId,
    workspace_id: String,
    profile: WorkspaceSummary,
    selected: bool,
}

impl Render for DraggedWorkspaceTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_preview(cx)
    }
}
