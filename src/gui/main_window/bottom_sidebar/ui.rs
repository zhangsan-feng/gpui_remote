use gpui_kit::{Div, Styled, div, px};

use gpui_kit::component::ThemeColor;

pub(in crate::gui::main_window) fn render(colors: &ThemeColor) -> Div {
    div()
        .w_full()
        .h(px(32.))
        .flex_none()
        .border_t_1()
        .border_color(colors.status_bar_border)
        .bg(colors.status_bar)
}
