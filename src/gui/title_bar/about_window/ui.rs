use crate::component::theme;
use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use super::core::build_time_text;

pub(super) fn render_about_window(cx: &Context<super::AboutWindow>) -> impl IntoElement {
    let colors = theme::CustomerUiTheme::colors(cx);
    v_flex()
        .id("about-window-content")
        .role(Role::Document)
        .size_full()
        .items_center()
        .justify_center()
        .bg(colors.background)
        .text_color(colors.text_color)
        .child(
            v_flex()
                .w(px(320.))
                .gap_3()
                .p_5()
                .rounded_xl()
                .border_1()
                .border_color(theme::CustomerUiTheme::border_color(cx))
                .bg(theme::CustomerUiTheme::panel_background(cx))
                .child(
                    div()
                        .id("about-window-build-time")
                        .role(Role::Paragraph)
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(build_time_text(crate::build_info::BUILD_TIME)),
                ),
        )
}
