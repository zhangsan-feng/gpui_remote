mod core;
mod external;
mod internal;
mod ui;

use anyhow::Error;
use gpui::*;
use gpui_component::input::{InputEvent, InputState};
use serde::Deserialize;

use crate::{component::draggable_list::DraggableList, domain::session::SessionProfile};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct ConnectSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct ConnectSftpSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct EditSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct DeleteSession(String);

pub struct SessionComponent {
    draggable_list: Entity<DraggableList>,
    sessions: Vec<SessionProfile>,
    core_err: Option<Error>,
    search_input: Entity<InputState>,
}

impl SessionComponent {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = SessionComponent {
            draggable_list: cx.new(move |_| DraggableList::new()),
            sessions: Vec::new(),
            core_err: None,
            search_input: cx.new(|cx| InputState::new(window, cx).placeholder("搜索会话")),
        };
        cx.subscribe(&this.search_input, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.refer_item(cx);
                cx.notify();
            }
        })
        .detach();
        this.start_subscribe(cx);
        if let Err(error) = this.reload_session(cx) {
            this.core_err = Some(error);
        }
        this
    }
}

impl Render for SessionComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}
