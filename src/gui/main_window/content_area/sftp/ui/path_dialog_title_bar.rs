use gpui_kit::component::*;
use gpui_kit::*;

pub(super) struct PathDialogTitleBar {
    title: SharedString,
}

impl PathDialogTitleBar {
    pub(super) fn new(
        title: impl Into<SharedString>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Self {
        Self {
            title: title.into(),
        }
    }

    fn render_close_button(&self, cx: &Context<Self>) -> AnyElement {
        let colors = cx.theme().colors;
        div()
            .id("sftp-path-dialog-titlebar-close")
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

    fn render_title_bar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close = self.render_close_button(cx);

        h_flex()
            .id("sftp-path-dialog-titlebar")
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
                    .id("sftp-path-dialog-titlebar-drag")
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
                            .child(self.title.clone()),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(cx.theme().border)
                    .gap_0()
                    .child(close),
            )
    }
}

impl Render for PathDialogTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_title_bar(window, cx)
    }
}
