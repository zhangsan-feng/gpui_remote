use gpui_kit::component::{ActiveTheme, v_flex};
use gpui_kit::*;

use super::core::build_time_text;

pub(super) fn render_about_window(cx: &Context<super::AboutWindow>) -> impl IntoElement {
    let colors = cx.theme().colors;
    v_flex()
        .id("about-window-content")
        .role(Role::Document)
        .size_full()
        .items_center()
        .justify_center()
        .bg(colors.background)
        .text_color(colors.foreground)
        .child(
            v_flex()
                .w(px(320.))
                .gap_3()
                .p_5()
                .rounded_xl()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().group_box)
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
