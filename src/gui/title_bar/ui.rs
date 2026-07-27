use gpui::prelude::FluentBuilder;
use gpui::*;

use super::AppTitleBar;
use crate::component::color::rgb_to_u32;

impl AppTitleBar {
    pub(super) fn window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        hover_color: Rgba,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(rgb_to_u32(73, 66, 92))
            .hover(move |style| style.bg(hover_color))
            .window_control_area(control)
            .when(cfg!(target_os = "linux"), move |this| {
                this.on_click(cx.listener(move |_, _, window, _| match control {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    WindowControlArea::Close => window.remove_window(),
                    _ => {}
                }))
            })
            .child(label)
            .into_any_element()
    }
}
