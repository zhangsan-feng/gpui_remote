mod core;
mod session;
mod terminal;
mod ui;
mod sftp;

use std::collections::HashMap;

use gpui::*;
use gpui_component::v_flex;

use crate::{
    component::{color::rgb_to_u32, draggable_list::DraggableList},
    domain::terminal::TerminalStatus,
};

use crate::global_state::read_global_state;
use core::{tab_statuses, workspace_state_data};
use session::WorkspaceSession;
use terminal::TerminalView;
use ui::render_empty_workspace;

pub struct Workspace {
    tabs: Entity<DraggableList>,
    workspace: Entity<WorkspaceSession>,
    terminal: Entity<TerminalView>,
    tab_statuses: HashMap<String, TerminalStatus>,
}

impl Workspace {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        terminal::init(cx);
        let global_state = read_global_state(cx);
        let workspace = cx.new(|cx| WorkspaceSession::new(global_state, cx));
        let terminal = cx.new(|cx| TerminalView::new(workspace.clone(), cx));
        let tabs = cx.new(|_| {
            let mut tabs = DraggableList::new();
            tabs.set_axis(Axis::Horizontal)
                .set_item_width(px(240.))
                .set_item_height(px(34.))
                .set_item_bg(rgb_to_u32(246, 243, 249))
                .set_item_hover_bg(rgb_to_u32(238, 232, 243));
            tabs
        });

        let terminal_updates = terminal.read(cx).status_updates();
        cx.spawn(async move |this, cx| {
            loop {
                terminal_updates.notified().await;
                if this
                    .update(cx, |this, cx| {
                        let (session_data, active_id) =
                            workspace_state_data(this.workspace.read(cx), this.terminal.read(cx));
                        let statuses = tab_statuses(&session_data);
                        if statuses != this.tab_statuses {
                            this.populate_tabs(session_data, active_id, cx);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        let mut this = Self {
            tabs,
            workspace,
            terminal,
            tab_statuses: HashMap::new(),
        };
        this.start_subscription(cx);
        let (session_data, active_id) =
            workspace_state_data(this.workspace.read(cx), this.terminal.read(cx));
        this.populate_tabs(session_data, active_id, cx);
        this
    }

    pub(super) fn start_subscription(&self, cx: &mut Context<Self>) {
        cx.subscribe(&self.workspace, |this, _, _event, cx| {
            let (session_data, active_id) =
                workspace_state_data(this.workspace.read(cx), this.terminal.read(cx));
            this.populate_tabs(session_data, active_id, cx);
            cx.notify();
        })
        .detach();
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_id = self
            .workspace
            .read(cx)
            .active_session_id()
            .map(str::to_owned);
        v_flex()
            .p_2()
            .gap_2()
            .size_full()
            .bg(rgb_to_u32(255, 255, 255))
            .child(
                div()
                    .w_full()
                    .h(px(45.))
                    .border_color(rgb_to_u32(225, 219, 230))
                    .bg(rgb_to_u32(246, 243, 249))
                    .child(self.tabs.clone()),
            )
            .child(match active_id {
                Some(_) => self.terminal.clone().into_any_element(),
                None => render_empty_workspace().into_any_element(),
            })
    }
}
