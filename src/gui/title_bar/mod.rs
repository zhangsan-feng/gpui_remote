mod core;
pub mod session_operation_window;
mod settings_operation_window;
mod ui;
mod mcp_opertaion_window;
mod tools_operation_window;

use crate::component::theme;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::DropdownMenu as _,
    ActiveTheme, IconName, Sizable,
};

actions!(title_bar, [CreateSession, OpenSettings]);

pub struct AppTitleBar;

impl AppTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let maximize_label = if window.is_maximized() { "❐" } else { "□" };
        let colors = cx.theme();
        h_flex()
            .id("app-title-bar")
            .on_action(cx.listener(Self::open_session_window))
            .on_action(cx.listener(Self::open_settings))
            .w_full()
            .h(px(42.))
            .flex_shrink_0()
            .border_b_1()
            .border_color(theme::border_color(cx))
            .bg(theme::title_background(cx))
            .child(
                h_flex()
                    .h_full()
                    .px_3()
                    .gap_2()
                    .child(
                        Button::new("session-menu")
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
                            .dropdown_menu(|menu, _, _| {
                                menu.menu_with_icon(
                                    "通用设置",
                                    IconName::Settings2,
                                    Box::new(OpenSettings),
                                )
                            }),
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
                        colors.accent,
                        cx,
                    ))
                    .child(self.window_button(
                        "window-max",
                        maximize_label,
                        WindowControlArea::Max,
                        colors.accent,
                        cx,
                    ))
                    .child(self.window_button(
                        "window-close",
                        "×",
                        WindowControlArea::Close,
                        colors.danger,
                        cx,
                    )),
            )
    }
}
