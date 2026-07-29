use gpui::*;
use gpui_component::{IconName, v_flex};

use crate::component::color::rgb_to_u32;
pub(super) fn render_empty_workspace() -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .text_color(rgb_to_u32(118, 109, 130))
        .child(div().text_size(px(36.)).child(IconName::SquareTerminal))
}
