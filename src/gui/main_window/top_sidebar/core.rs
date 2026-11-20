use gpui_kit::*;

use crate::gui::{
    service_window::open_services_window, session_window::open_new_session_window,
    settings_window::open_settings_window,
};

use super::{AppTitleBar, CreateSession, OpenSettings, about_window::open_about_window};

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

    pub(super) fn open_services(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_services_window(window, cx);
    }

    pub(super) fn open_about(
        &mut self,
        _: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        open_about_window(window, cx);
    }
}
