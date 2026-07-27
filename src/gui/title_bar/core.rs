use gpui::*;
use gpui_component::WindowExt;

use super::{
    AppTitleBar, CreateSession, OpenSettings, session_operation_window::open_new_session_window,
};
use crate::component::color::rgb_to_u32;

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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, |dialog, _, _| {
            dialog.title("设置").child(
                div()
                    .py_3()
                    .text_sm()
                    .text_color(rgb_to_u32(107, 99, 120))
                    .child("通用设置将在后续功能中接入。"),
            )
        });
    }
}
