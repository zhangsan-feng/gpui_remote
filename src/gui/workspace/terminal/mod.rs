mod core;
mod external;
mod internal;
mod ui;

use std::{collections::HashMap, sync::Arc};

use gpui::*;
use serde::Deserialize;
use tokio::sync::Notify;

use core::TerminalRuntime;
use internal::{TerminalScrollHandle, TerminalSelection};

pub(in crate::gui::workspace) use external::encode_agent_key;

const TERMINAL_KEY_CONTEXT: &str = "Terminal";
const TERMINAL_FONT_FAMILY: &str = "Consolas";
const TERMINAL_FONT_SIZE: f32 = 13.0;
const TERMINAL_LINE_HEIGHT: f32 = 19.0;
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
    terminals: HashMap<String, TerminalRuntime>,
    selected_workspace_id: Option<String>,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
    focus: FocusHandle,
    list_state: ListState,
    listed_workspace_id: Option<String>,
    last_pty_size: Option<(String, u32, u32)>,
    observed_revision: Option<(String, u64)>,
    selection: Option<TerminalSelection>,
    selecting_text: bool,
    scroll_handle: TerminalScrollHandle,
}

pub(in crate::gui::workspace) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some(TERMINAL_KEY_CONTEXT)),
        KeyBinding::new("ctrl-v", PasteTerminal, Some(TERMINAL_KEY_CONTEXT)),
    ]);
}

impl TerminalView {
    pub(in crate::gui::workspace) fn new(cx: &mut Context<Self>) -> Self {
        let updates = Arc::new(Notify::new());
        let status_updates = Arc::new(Notify::new());
        let list_state = ListState::new(0, ListAlignment::Top, px(256.))
            .with_uniform_item_height(px(TERMINAL_LINE_HEIGHT));
        list_state.set_follow_mode(FollowMode::Tail);

        let this = Self {
            terminals: HashMap::new(),
            selected_workspace_id: None,
            updates,
            status_updates,
            focus: cx.focus_handle(),
            list_state,
            listed_workspace_id: None,
            last_pty_size: None,
            observed_revision: None,
            selection: None,
            selecting_text: false,
            scroll_handle: TerminalScrollHandle::default(),
        };
        this.start_model_watcher(cx);
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
