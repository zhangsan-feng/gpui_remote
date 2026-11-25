mod core;
mod external;
mod internal;
mod ui;

use std::{collections::HashMap, sync::Arc};

use gpui_kit::*;
use serde::Deserialize;

use crate::{
    domain::terminal::{MAX_TERMINAL_FONT_SIZE, MIN_TERMINAL_FONT_SIZE},
    gui::component::theme::TerminalAppearance,
};
use core::TerminalModel;
use internal::{TerminalPoint, TerminalScrollHandle, TerminalSelection};

const TERMINAL_KEY_CONTEXT: &str = "Terminal";
const TERMINAL_FONT_FAMILY: &str = "Consolas";
const TERMINAL_LINE_PADDING: f32 = 6.0;
const TERMINAL_TEXT_HORIZONTAL_PADDING: f32 = 20.0;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = terminal, no_json)]
struct SendTab;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = terminal, no_json)]
struct CopyTerminal;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = terminal, no_json)]
struct PasteTerminal;

pub(super) struct TerminalView {
    models: HashMap<String, Arc<TerminalModel>>,
    selected_workspace_id: Option<String>,
    focus: FocusHandle,
    focus_pending: bool,
    font_size: u8,
    list_state: ListState,
    listed_workspace_id: Option<String>,
    last_pty_size: Option<(String, u32, u32)>,
    observed_gui_snapshot_version: Option<(String, u64)>,
    selection: Option<TerminalSelection>,
    selection_origin: Option<(String, TerminalPoint)>,
    selecting_text: bool,
    scroll_handle: TerminalScrollHandle,
}

pub(in crate::gui::main_window::content_area) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some(TERMINAL_KEY_CONTEXT)),
        KeyBinding::new("ctrl-v", PasteTerminal, Some(TERMINAL_KEY_CONTEXT)),
    ]);
}

impl TerminalView {
    fn start_subscribe(&self, cx: &mut Context<Self>) {
        let mut refresh = crate::event_bus::EVENT_BUS
            .refresh_gui
            .subscribe(crate::event_bus::refresh_gui::GuiComponent::Terminal);
        cx.spawn(async move |this, cx| {
            while refresh.changed().await.is_ok() {
                let scope = refresh.take_pending();
                if this
                    .update(cx, |this, cx| {
                        if scope.all {
                            let snapshot = crate::data_context::DATA_CONTEXT.workspace_snapshot();
                            this.apply_snapshot(&snapshot, cx);
                        } else {
                            for workspace_id in scope.workspace_ids {
                                this.refresh_session_from_context(&workspace_id, cx);
                            }
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(in crate::gui::main_window::content_area) fn new(cx: &mut Context<Self>) -> Self {
        let font_size = TerminalAppearance::font_size(cx);
        let list_state = ListState::new(0, ListAlignment::Top, px(256.));
        list_state.set_follow_mode(FollowMode::Tail);
        cx.observe_global::<TerminalAppearance>(|this, cx| {
            this.set_font_size(TerminalAppearance::font_size(cx), cx);
        })
        .detach();

        let this = Self {
            models: HashMap::new(),
            selected_workspace_id: None,
            focus: cx.focus_handle(),
            focus_pending: false,
            font_size,
            list_state,
            listed_workspace_id: None,
            last_pty_size: None,
            observed_gui_snapshot_version: None,
            selection: None,
            selection_origin: None,
            selecting_text: false,
            scroll_handle: TerminalScrollHandle::default(),
        };
        this.start_subscribe(cx);
        this
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(window, cx)
    }
}
