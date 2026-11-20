use std::rc::Rc;

use gpui_kit::{component::Root, *};

use crate::{
    application::session::model::SessionSummary, domain::session::SessionGroup,
    gui::component::window::window_center_options,
};

use super::MoveSessionWindow;

pub(in crate::gui) fn open(
    session: SessionSummary,
    groups: Vec<SessionGroup>,
    on_moved: impl Fn(&mut App, Option<&str>, Option<&str>) + 'static,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<()> {
    let mut options = window_center_options(window, size(px(480.), px(510.)));
    options.titlebar = Some(TitlebarOptions {
        title: Some("移动到分组".into()),
        ..Default::default()
    });
    options.is_minimizable = false;
    cx.open_window(options, move |window, cx| {
        window.on_window_should_close(cx, |window, _| {
            window.remove_window();
            false
        });
        let form =
            cx.new(|cx| MoveSessionWindow::new(session, groups, Rc::new(on_moved), window, cx));
        cx.new(|cx| Root::new(form, window, cx))
    })?;
    Ok(())
}
