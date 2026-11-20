mod core;
mod external;
mod ui;

use std::rc::Rc;

use gpui_kit::{
    component::input::{InputEvent, InputState},
    *,
};

use crate::{application::session::model::SessionSummary, domain::session::SessionGroup};

pub(super) use external::open;

type MoveCallback = dyn Fn(&mut App, Option<&str>, Option<&str>);

struct MoveSessionWindow {
    session: SessionSummary,
    groups: Vec<SessionGroup>,
    on_moved: Rc<MoveCallback>,
    selected: Option<String>,
    name: Entity<InputState>,
    busy: bool,
    error: Option<String>,
}

impl MoveSessionWindow {
    fn new(
        session: SessionSummary,
        groups: Vec<SessionGroup>,
        on_moved: Rc<MoveCallback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let selected = session.group_id.clone();
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("新分组名称"));
        cx.subscribe(&name, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })
        .detach();
        Self {
            session,
            groups,
            on_moved,
            selected,
            name,
            busy: false,
            error: None,
        }
    }
}

impl Render for MoveSessionWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}
