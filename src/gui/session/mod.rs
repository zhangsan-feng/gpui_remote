mod core;
mod ui;

use anyhow::Error;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::input::InputState;
use gpui_component::*;
use serde::Deserialize;

use crate::{
    component::{color::rgb_to_u32, draggable_list::DraggableList, theme},
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
            match event {
                GlobalEvent::CreateSession
                | GlobalEvent::UpdateSession
                | GlobalEvent::ThemeChanged => {}
                _ => return,
            }
            if let Err(error) = this.reload_session(cx) {
                this.core_err = Some(error);
                cx.notify();
            }
        })
        .detach();
    }
}

impl Render for SessionComponent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .on_action(cx.listener(Self::create_active_session))
            .on_action(cx.listener(Self::edit_session))
            .on_action(cx.listener(Self::delete_session))
            .h_full()
            .w_full()
            .flex_shrink_0()
            .border_r_1()
            .border_color(colors.sidebar_border)
            .bg(theme::sidebar_color(cx))
            .text_color(contrast_text(theme::sidebar_base_color(cx)))
            .child(
                h_flex()
                    .h(px(48.))
                    .px_4()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.sidebar_border)
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
                        .bg(colors.danger)
                        .text_xs()
                        .text_color(colors.danger_foreground)
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

fn contrast_text(background: Hsla) -> Hsla {
    if background.l < 0.48 {
        rgb_to_u32(248, 250, 252).into()
    } else {
        rgb_to_u32(30, 41, 59).into()
    }
}
