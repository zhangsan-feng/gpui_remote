mod core;
mod ui;

use anyhow::Error;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::InputState;
use gpui_component::*;
use serde::Deserialize;

use crate::{
    component::{color::rgb_to_u32, draggable_list::DraggableList},
    domain::session::SessionProfile,
    global_state::{GlobalEvent, read_global_state},
};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct ConnectSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct EditSession(String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = left_session, no_json)]
struct DeleteSession(String);

pub struct SessionComponent {
    draggable_list: Entity<DraggableList>,
    select_session: Option<ElementId>,
    sessions: Vec<SessionProfile>,
    core_err: Option<Error>,
    search_input: Entity<InputState>,
}

impl SessionComponent {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = SessionComponent {
            draggable_list: cx.new(move |_| DraggableList::new()),
            select_session: None,
            sessions: Vec::new(),
            core_err: None,
            search_input: cx.new(|cx| InputState::new(window, cx)),
        };
        this.start_subscribe(cx);
        if let Err(error) = this.reload_session(cx) {
            this.core_err = Some(error);
        }
        this
    }

    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_events = read_global_state(cx);
        cx.subscribe(&global_events, |this, _, event, cx| {
            if matches!(
                event,
                GlobalEvent::CreateSession | GlobalEvent::UpdateSession
            ) {
                if let Err(error) = this.reload_session(cx) {
                    this.core_err = Some(error);
                    cx.notify();
                }
            }
        })
        .detach();
    }
}

impl Render for SessionComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .on_action(cx.listener(Self::create_active_session))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::delete_session))
            .h_full()
            .w_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb_to_u32(230, 224, 235))
            .bg(rgb_to_u32(249, 247, 251))
            .child(
                h_flex()
                    .h(px(48.))
                    .px_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(rgb_to_u32(235, 230, 239))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("会话"),
                    ),
            )
            .when_some(self.core_err.as_ref(), |this, error| {
                this.child(
                    div()
                        .m_2()
                        .p_2()
                        .rounded_md()
                        .bg(rgb_to_u32(254, 242, 242))
                        .text_xs()
                        .text_color(rgb_to_u32(185, 28, 28))
                        .child(error.to_string()),
                )
            })
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.draggable_list.clone()),
            )
    }
}
