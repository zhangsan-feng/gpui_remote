use gpui::*;
use gpui_component::{
    ActiveTheme,
    color_picker::{ColorPickerEvent, ColorPickerState},
    h_flex,
    scroll::ScrollableElement,
    slider::{SliderEvent, SliderState},
    v_flex,
};

mod background;
mod color;
mod core;
mod ui;

pub(crate) use core::open_settings_window;

pub struct SettingsOperationWindow {
    color_picker: Entity<ColorPickerState>,
    sidebar_color_picker: Entity<ColorPickerState>,
    terminal_color_picker: Entity<ColorPickerState>,
    sidebar_opacity: Entity<SliderState>,
    terminal_opacity: Entity<SliderState>,
    wallpaper_opacity: Entity<SliderState>,
    wallpaper_error: Option<String>,
}

impl SettingsOperationWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(crate::component::theme::custom_accent(cx))
        });
        let sidebar_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(crate::component::theme::sidebar_base_color(cx))
        });
        let terminal_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(crate::component::theme::terminal_base_color(cx))
        });
        let sidebar_opacity_value = crate::component::theme::sidebar_opacity(cx) * 100.;
        let terminal_opacity_value = crate::component::theme::terminal_opacity(cx) * 100.;
        let wallpaper_opacity_value = crate::component::theme::wallpaper_opacity(cx) * 100.;
        let sidebar_opacity = cx.new(move |_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(sidebar_opacity_value)
        });
        let terminal_opacity = cx.new(move |_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(terminal_opacity_value)
        });
        let wallpaper_opacity = cx.new(move |_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(wallpaper_opacity_value)
        });
        cx.subscribe(
            &color_picker,
            |_, _, event: &ColorPickerEvent, cx| match event {
                ColorPickerEvent::Change(Some(color)) => {
                    crate::component::theme::select_custom(*color, cx);
                }
                ColorPickerEvent::Change(None) => {}
            },
        )
        .detach();
        cx.subscribe(
            &sidebar_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    crate::component::theme::set_sidebar_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(
            &terminal_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    crate::component::theme::set_terminal_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(&sidebar_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::set_sidebar_opacity(value.start() / 100., cx);
            }
        })
        .detach();
        cx.subscribe(&terminal_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::set_terminal_opacity(value.start() / 100., cx);
            }
        })
        .detach();
        cx.subscribe(&wallpaper_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::set_wallpaper_opacity(value.start() / 100., cx);
            }
        })
        .detach();
        Self {
            color_picker,
            sidebar_color_picker,
            terminal_color_picker,
            sidebar_opacity,
            terminal_opacity,
            wallpaper_opacity,
            wallpaper_error: None,
        }
    }
}

impl Render for SettingsOperationWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .size_full()
            .items_stretch()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(self.sidebar(cx))
            .child(
                v_flex().flex_1().min_w_0().bg(colors.background).child(
                    v_flex()
                        .flex_1()
                        .overflow_y_scrollbar()
                        .p_6()
                        .gap_5()
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xl()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("外观"),
                                )
                                .child(
                                    div().text_sm().text_color(colors.muted_foreground).child(
                                        "选择预设主题或自定义主色，修改后立即生效并自动保存。",
                                    ),
                                ),
                        )
                        .child(self.region_appearance_panel(cx))
                        .child(self.wallpaper_panel(cx))
                        .child(self.custom_color_panel(cx))
                        .child(
                            v_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("主题预设"),
                                )
                                .child(
                                    div().grid().grid_cols(3).gap_3().children(
                                        crate::component::theme::AppTheme::ALL
                                            .map(|theme| self.theme_card(theme, cx)),
                                    ),
                                ),
                        ),
                ),
            )
    }
}
