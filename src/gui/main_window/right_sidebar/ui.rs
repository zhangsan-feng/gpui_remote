use gpui_kit::{Div, Styled, div, px};

use gpui_kit::component::ThemeColor;

pub(in crate::gui::main_window) fn render(colors: &ThemeColor) -> Div {
    div()
        .w(px(56.))
        .h_full()
        .flex_none()
        .border_l_1()
        .border_color(colors.sidebar_border)
        .bg(colors.sidebar)
}
