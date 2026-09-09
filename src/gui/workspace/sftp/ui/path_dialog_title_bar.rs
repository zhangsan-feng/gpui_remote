use crate::component::theme;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
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

    fn render_window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .text_color(colors.text_color)
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
                    .text_color(colors.text_color)
                    .child(label),
            )
            .into_any_element()
    }

    fn render_close_button(&self, cx: &Context<Self>) -> AnyElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        div()
            .id("sftp-path-dialog-titlebar-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .text_color(colors.text_color)
            .hover(|style| style.bg(colors.hover_background))
            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
            .child(
                div()
                    .text_size(px(14.))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(colors.text_color)
                    .child("×"),
            )
            .into_any_element()
    }

    fn render_title_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let minimize = self.render_window_button(
            "sftp-path-dialog-titlebar-minimize",
            "−",
            WindowControlArea::Min,
            theme::CustomerUiTheme::colors(cx).hover_background,
            cx,
        );
        let maximize = self.render_window_button(
            "sftp-path-dialog-titlebar-maximize",
            if window.is_maximized() { "❐" } else { "□" },
            WindowControlArea::Max,
            theme::CustomerUiTheme::colors(cx).hover_background,
            cx,
        );
        let close = self.render_close_button(cx);

        h_flex()
            .id("sftp-path-dialog-titlebar")
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
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
                            .text_color(theme::CustomerUiTheme::colors(cx).text_color)
                            .child(self.title.clone()),
                    ),
            )
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    .border_l_1()
                    .border_color(theme::CustomerUiTheme::border_color(cx))
                    .gap_0()
                    .children(vec![minimize, maximize, close]),
            )
    }
}

impl Render for PathDialogTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_title_bar(window, cx)
    }
}
