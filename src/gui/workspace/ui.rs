use gpui::*;
use gpui_component::{IconName, v_flex};

pub(super) fn render_empty_workspace() -> Div {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .child(div().text_size(px(36.)).child(IconName::SquareTerminal))
}
