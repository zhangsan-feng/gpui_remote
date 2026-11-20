mod about_window;
mod core;
mod ui;

use gpui_kit::*;

gpui_kit::actions!(title_bar, [CreateSession, OpenSettings, OpenTools]);

pub struct AppTitleBar;

impl AppTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(window, cx)
    }
}
