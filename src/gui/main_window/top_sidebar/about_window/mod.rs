use gpui_kit::*;

mod core;
mod external;
mod ui;

pub(super) use external::open_about_window;

pub(super) struct AboutWindow;

impl Render for AboutWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::render_about_window(cx)
    }
}
