use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub struct WindowCustomTitleBar {}

impl WindowCustomTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self {}
    }
    fn render_window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        let colors = cx.theme().colors;
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().group_box)
            .text_color(colors.foreground)
            .hover(|style| style.bg(hover_color))
            .window_control_area(control)
            .when(cfg!(target_os = "linux"), move |this| {
                this.on_click(cx.listener(move |_, _, window, _| match control {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    _ => {}
                }))
            })
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(colors.foreground)
                    .child(label),
            )
            .into_any_element()
    }

    fn render_close_button(&self, cx: &Context<Self>) -> AnyElement {
        let colors = cx.theme().colors;
        div()
            .id("custom-titlebar-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().group_box)
            .text_color(colors.foreground)
            .hover(|style| style.bg(colors.list_hover))
            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(colors.foreground)
                    .child("×"),
            )
            .into_any_element()
    }

    fn render_title_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let minimize = self.render_window_button(
            "custom-titlebar-minimize",
            "−",
            WindowControlArea::Min,
            cx.theme().colors.list_hover,
            cx,
        );
        let maximize = self.render_window_button(
            "custom-titlebar-maximize",
            if window.is_maximized() { "❐" } else { "□" },
            WindowControlArea::Max,
            cx.theme().colors.list_hover,
            cx,
        );
        let close = self.render_close_button(cx);

        h_flex()
            .id("custom-titlebar")
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().group_box)
            .child(
                h_flex()
                    .id("custom-titlebar-drag")
                    .h_full()
                    .flex_1()
                    .items_center()
                    .px_4()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .px_2()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(cx.theme().colors.foreground)
                            .child(""),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .gap_0()
                    .children(vec![minimize, maximize, close]),
            )
    }
}

impl Render for WindowCustomTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_title_bar(window, cx)
    }
}
