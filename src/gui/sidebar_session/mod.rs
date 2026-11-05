mod core;
mod external;
mod internal;
mod ui;

use anyhow::Error;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;
use serde::Deserialize;

use crate::application::session::model::SessionSummary;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct ConnectSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct ConnectSftpSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct OpenSshTunnel(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct CloseSshTunnel(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct EditSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct DeleteSession(String);

pub struct SessionComponent {
    gui_event_task: Option<Task<()>>,
    gui_scope: EntityId,
    scroll: ScrollHandle,
    selected_id: Option<String>,
    drop_target: Option<(String, bool)>,
    sessions: Vec<SessionSummary>,
    core_err: Option<Error>,
    search_input: Entity<InputState>,
    refresh_generation: u64,
}

impl SessionComponent {
    pub fn new(gui_scope: EntityId, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = SessionComponent {
            gui_event_task: None,
            gui_scope,
            scroll: ScrollHandle::new(),
            selected_id: None,
            drop_target: None,
            sessions: Vec::new(),
            core_err: None,
            search_input: cx.new(|cx| InputState::new(window, cx).placeholder("搜索会话")),
            refresh_generation: 0,
        };
        cx.subscribe(&this.search_input, |_this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
        this.start_subscribe(cx);
        this.refresh_sessions(cx);
        this
    }
}

impl Render for SessionComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}

impl SessionComponent {
    fn start_subscribe(&mut self, cx: &mut Context<Self>) {
        use crate::event_bus::{
            EVENT_BUS,
            gui_event::{GuiEvent, GuiTarget},
        };
        let mut events = EVENT_BUS
            .gui_event
            .subscribe(self.gui_scope, GuiTarget::SidebarSession);
        self.gui_event_task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = events.receiver.recv().await {
                if this
                    .update(cx, |this, cx| match event {
                        GuiEvent::CreateSession | GuiEvent::UpdateSession => {
                            this.refresh_sessions(cx)
                        }
                        GuiEvent::ThemeColorChanged => {
                            cx.notify();
                        }
                        _ => {}
                    })
                    .is_err()
                {
                    break;
                }
            }
        }));
    }
}

#[derive(Clone)]
struct DraggedSession {
    owner: EntityId,
    id: String,
    name: String,
    subtitle: String,
}

impl Render for DraggedSession {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_preview(cx)
    }
}
