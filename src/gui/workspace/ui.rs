use gpui::*;
use gpui_component::{IconName, v_flex};

use crate::component::color::rgb_to_u32;
use crate::domain::terminal::TerminalStatus;

use super::{
    Workspace,
    core::tab_statuses,
    session::{OpenedWorkspaceSession, workspace_tab},
};

impl Workspace {
    pub(super) fn populate_tabs(
        &mut self,
        sessions: Vec<(OpenedWorkspaceSession, TerminalStatus)>,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.tab_statuses = tab_statuses(&sessions);
        let workspace = self.workspace.clone();
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.clear(tabs_cx);
            for (opened_session, status) in sessions {
                let click_workspace = workspace.clone();
                let active = active_id.as_deref() == Some(opened_session.session.id.as_str());
                tabs.child(opened_session.session.id.clone(), move || {
                    workspace_tab(
                        opened_session.session.clone(),
                        opened_session.profile.clone(),
                        status.clone(),
                        active,
                        click_workspace.clone(),
                    )
                });
            }
        });
    }
}

pub(super) fn render_empty_workspace() -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .text_color(rgb_to_u32(118, 109, 130))
        .child(div().text_size(px(36.)).child(IconName::SquareTerminal))
}
