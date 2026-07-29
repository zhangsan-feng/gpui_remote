use super::{
    AppTitleBar, CreateSession, OpenSettings, session_operation_window::open_new_session_window,
    settings_operation_window::open_settings_window,
};
use gpui::*;

impl AppTitleBar {
    pub(super) fn open_session_window(
        &mut self,
        _: &CreateSession,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_new_session_window(cx);
    }

    pub(super) fn open_settings(
        &mut self,
        _: &OpenSettings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_settings_window(cx);
    }
}
