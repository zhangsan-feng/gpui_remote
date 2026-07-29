mod buffer;
mod core;
mod keyboard;
mod pty;
mod scroll;
mod selection;
mod ssh;
mod terminal_render;
mod ui;

use std::{collections::HashMap, sync::Arc};

use gpui::*;
use serde::Deserialize;
use tokio::sync::Notify;

use crate::global_state::{GlobalEvent, read_global_state};

use pty::TerminalRuntime;
use scroll::TerminalScrollHandle;
use selection::TerminalSelection;

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

fn terminal_cell_width(window: &Window) -> Pixels {
    let run = TextRun {
        len: 1,
        font: Font {
            family: TERMINAL_FONT_FAMILY.into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let width = window
        .text_system()
        .layout_line("0", px(TERMINAL_FONT_SIZE), &[run], None)
        .width;
    if width > Pixels::ZERO { width } else { px(8.) }
}

impl TerminalView {
    pub(in crate::gui::workspace) fn new(cx: &mut Context<Self>) -> Self {
        let updates = Arc::new(Notify::new());
        let status_updates = Arc::new(Notify::new());
        let list_state = ListState::new(0, ListAlignment::Top, px(256.))
            .with_uniform_item_height(px(TERMINAL_LINE_HEIGHT));
        list_state.set_follow_mode(FollowMode::Tail);

        let terminal_updates = updates.clone();
        cx.spawn(async move |this, cx| {
            loop {
                terminal_updates.notified().await;
                if this
                    .update(cx, |this, cx| this.notify_if_model_changed(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

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
        this.start_subscribe(cx);
        this
    }

    fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);

        cx.subscribe(&global_state, |this, _, event, cx| {
            match event {
                GlobalEvent::CreateActiveSessionTerminal {
                    workspace_id,
                    profile,
                } => this.connect(workspace_id.clone(), profile.clone()),
                GlobalEvent::CloseActiveSession(workspace_id) => this.close(workspace_id),
                _ => return,
            }
            this.reset_active_view();
            cx.notify();
        })
        .detach();
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
