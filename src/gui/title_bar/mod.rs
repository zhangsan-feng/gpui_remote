mod about_window;
mod core;
mod services_operation_window;
pub mod session_operation_window;
mod settings_operation_window;
mod ui;

use gpui_kit::component::{
    IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::DropdownMenu as _,
};
use gpui_kit::*;

gpui_kit::actions!(title_bar, [CreateSession, OpenSettings, OpenTools]);

pub struct AppTitleBar;

impl AppTitleBar {
    pub fn new(_: &mut Window, _: &mut Context<Self>) -> Self {
        Self
    }
}

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let maximize_label = if window.is_maximized() { "❐" } else { "□" };
        let palette = ui::title_bar_palette(cx);
        h_flex()
            .id("app-title-bar")
            .on_action(cx.listener(Self::open_session_window))
            .on_action(cx.listener(Self::open_settings))
            .w_full()
            .h(px(38.))
            .flex_shrink_0()
            .border_b_1()
            .border_color(palette.border)
            .bg(palette.background)
            .child(
                h_flex()
                    .h_full()
                    .px_3()
                    .gap_2()
                    .text_color(palette.foreground)
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
                            .dropdown_menu(|menu, _, _| {
                                menu.menu_with_icon(
                                    "通用设置",
                                    IconName::Settings2,
                                    Box::new(OpenSettings),
                                )
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
                    .border_color(palette.border)
                    .child(self.window_button(
                        "window-min",
                        "−",
                        WindowControlArea::Min,
                        palette.foreground,
                        palette.control_hover,
                        cx,
                    ))
                    .child(self.window_button(
                        "window-max",
                        maximize_label,
                        WindowControlArea::Max,
                        palette.foreground,
                        palette.control_hover,
                        cx,
                    ))
                    .child(self.close_button(palette.foreground, palette.close_hover, cx)),
            )
    }
}
