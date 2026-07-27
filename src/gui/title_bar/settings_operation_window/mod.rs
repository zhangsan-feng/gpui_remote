use gpui::*;
use gpui_component::*;

mod core;
mod ui;
mod color;

mod theme;

pub  struct SettingsOperationWindow {}

impl SettingsOperationWindow {
    fn new() -> SettingsOperationWindow {
        SettingsOperationWindow {}
    }
}


impl Render for SettingsOperationWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}