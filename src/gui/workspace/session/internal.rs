use gpui::{Context, ElementId};
use gpui_component::ActiveTheme;

use crate::{component::theme, domain::terminal::TerminalStatus};

use super::{WorkspaceSession, ui::workspace_tab};

impl WorkspaceSession {
    pub(super) fn rebuild_tabs(&mut self, cx: &mut Context<Self>) {
        let sessions = self.sessions.clone();
        let statuses = self.statuses.clone();
        let selected_id = self.selected_id.clone();
        let workspace = cx.entity();
        let colors = cx.theme().colors;
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.clear(tabs_cx);
            for opened_session in sessions {
                let workspace = workspace.clone();
                let status = statuses
                    .get(&opened_session.id)
                    .cloned()
                    .unwrap_or(TerminalStatus::Connecting);
                tabs.child(opened_session.id.clone(), move || {
                    workspace_tab(
                        opened_session.id.clone(),
                        opened_session.profile.clone(),
                        status.clone(),
                        workspace.clone(),
                        colors,
                    )
                });
            }
            if let Some(selected_id) = selected_id {
                let selected_id = ElementId::from(selected_id);
                tabs.set_selected_id(&selected_id, tabs_cx);
            }
        });
    }

    pub(super) fn reset_tab_style(&mut self, cx: &mut Context<Self>) {
        let tab_bar = theme::styles(cx).tab_bar.into();
        let selected_background = cx.theme().sidebar_accent.into();
        let list_hover = cx.theme().list_hover.into();
        self.tabs.update(cx, move |tabs, tabs_cx| {
            tabs.set_item_bg(tab_bar);
            tabs.set_item_selected_bg(selected_background);
            tabs.set_item_hover_bg(list_hover);
            tabs_cx.notify();
        });
    }

    pub(super) fn select_tab(&self, id: &str, cx: &mut Context<Self>) {
        let id = ElementId::from(id.to_owned());
        self.tabs.update(cx, |tabs, tabs_cx| {
            tabs.set_selected_id(&id, tabs_cx);
        });
    }
}
