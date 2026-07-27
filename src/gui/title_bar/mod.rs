mod core;
pub mod session_operation_window;
mod ui;
mod settings_operation_window;

use gpui::*;
use gpui_component::{
    IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::DropdownMenu as _,
};

use crate::component::color::rgb_to_u32;

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
        h_flex()
            .id("app-title-bar")
            .on_action(cx.listener(Self::open_session_window))
            .on_action(cx.listener(Self::open_settings))
            .w_full()
            .h(px(42.))
            .flex_shrink_0()
            .border_b_1()
            .border_color(rgb_to_u32(230, 224, 235))
            .bg(rgb_to_u32(250, 247, 252))
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
                    .border_color(rgb_to_u32(230, 224, 235))
                    .child(self.window_button(
                        "window-min",
                        "−",
                        WindowControlArea::Min,
                        rgb_to_u32(232, 225, 239),
                        cx,
                    ))
                    .child(self.window_button(
                        "window-max",
                        maximize_label,
                        WindowControlArea::Max,
                        rgb_to_u32(232, 225, 239),
                        cx,
                    ))
                    .child(self.window_button(
                        "window-close",
                        "×",
                        WindowControlArea::Close,
                        rgb_to_u32(254, 202, 202),
                        cx,
                    )),
            )
    }
}
