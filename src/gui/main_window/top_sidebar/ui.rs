use crate::gui::settings_window::open_settings_window;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::DropdownMenu as _,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::{AppTitleBar, CreateSession};

impl AppTitleBar {
    pub(super) fn window_button(
        &self,
        id: &'static str,
        label: &'static str,
        control: WindowControlArea,
        foreground: Hsla,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(foreground)
            .hover(move |style| style.bg(hover_color))
            .window_control_area(control)
            .when(cfg!(target_os = "linux"), move |this| {
                this.on_click(cx.listener(move |_, _, window, _| match control {
                    WindowControlArea::Min => window.minimize_window(),
                    WindowControlArea::Max => window.zoom_window(),
                    _ => {}
                }))
            })
            .child(label)
            .into_any_element()
    }

    pub(super) fn close_button(
        &self,
        foreground: Hsla,
        hover_color: Hsla,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id("window-close")
            .size(px(34.))
            .flex()
            .items_center()
            .justify_center()
            .text_color(foreground)
            .hover(move |style| style.bg(hover_color))
            .on_click(cx.listener(|_, _, window, _| {
                log::debug!("Main window custom close clicked: action=remove_window");
                window.remove_window();
                log::debug!("Main window custom close handled: action=remove_window");
            }))
            .child("×")
            .into_any_element()
    }
}

impl AppTitleBar {
    pub(super) fn render_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let maximize_label = if window.is_maximized() { "❐" } else { "□" };
        let colors = cx.theme().colors;
        h_flex()
            .id("app-title-bar")
            .on_action(cx.listener(Self::open_session_window))
            .on_action(cx.listener(Self::open_settings))
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .border_b_1()
            .border_color(colors.title_bar_border)
            .bg(colors.title_bar)
            .child(
                h_flex()
                    .h_full()
                    .px_3()
                    .gap_2()
                    .text_color(colors.foreground)
                    .child(
                        img(crate::gui::component::assets::APP_ICON)
                            .size(px(18.))
                            .flex_shrink_0(),
                    )
                    .child(
                        Button::new("top_session-menu")
                            .ghost()
                            .small()
                            .label("会话")
                            .dropdown_menu(|menu, _, _| {
                                menu.menu_with_icon(
                                    "新建会话",
                                    IconName::Plus,
                                    Box::new(CreateSession),
                                )
                            }),
                    )
                    .child(
                        Button::new("settings-menu")
                            .ghost()
                            .small()
                            .label("设置")
                            .on_click(|_, window, cx| {
                                open_settings_window(window, cx);
                            }),
                    )
                    .child(
                        Button::new("services-menu")
                            .ghost()
                            .small()
                            .label("服务")
                            .on_click(cx.listener(Self::open_services)),
                    )
                    .child(
                        Button::new("about-menu")
                            .ghost()
                            .small()
                            .label("关于")
                            .on_click(cx.listener(Self::open_about)),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .flex_1()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                h_flex()
                    .h_full()
                    .border_l_1()
                    .border_color(colors.title_bar_border)
                    .child(self.window_button(
                        "window-min",
                        "−",
                        WindowControlArea::Min,
                        colors.foreground,
                        colors.list_hover,
                        cx,
                    ))
                    .child(self.window_button(
                        "window-max",
                        maximize_label,
                        WindowControlArea::Max,
                        colors.foreground,
                        colors.list_hover,
                        cx,
                    ))
                    .child(self.close_button(colors.foreground, colors.danger.opacity(0.16), cx)),
            )
    }
}
