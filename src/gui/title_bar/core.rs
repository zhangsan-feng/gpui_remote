use super::{
    AppTitleBar, CreateSession, OpenSettings, about_dialog::open_about_dialog,
    session_operation_window::open_new_session_window,
    settings_operation_window::open_settings_window,
};
use gpui_kit::*;

impl AppTitleBar {
    pub(super) fn open_session_window(
        &mut self,
        _: &CreateSession,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_new_session_window(window, cx);
    }

    pub(super) fn open_settings(
        &mut self,
        _: &OpenSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(window, cx);
    }

    pub(super) fn open_about(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_about_dialog(window, cx);
    }
}
