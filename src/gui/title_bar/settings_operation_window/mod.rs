use gpui::*;
use gpui_component::{
    color_picker::{ColorPickerEvent, ColorPickerState},
    slider::{SliderEvent, SliderState},
    ActiveTheme,
};

use crate::component::theme;

mod core;
mod external;
mod internal;
mod ui;

pub(crate) use external::open_settings_window;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Theme,
    Wallpaper,
}

pub struct SettingsOperationWindow {
    color_picker: Entity<ColorPickerState>,
    font_color_picker: Entity<ColorPickerState>,
    background_color_picker: Entity<ColorPickerState>,
    hover_color_picker: Entity<ColorPickerState>,
    selected_color_picker: Entity<ColorPickerState>,
    wallpaper_opacity: Entity<SliderState>,
    window_opacity: Entity<SliderState>,
    wallpaper_error: Option<String>,
    active_section: SettingsSection,
}

impl SettingsOperationWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(crate::component::theme::custom_accent(cx))
        });
        let font_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                crate::component::theme::font_color(cx).unwrap_or(cx.theme().foreground),
            )
        });
        let background_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                crate::component::theme::background_color(cx).unwrap_or(cx.theme().background),
            )
        });
        let hover_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(theme::hover_color(cx).unwrap_or(theme::styles(cx).hover))
        });
        let selected_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(theme::selected_color(cx).unwrap_or(theme::styles(cx).selected))
        });
        let wallpaper_opacity_value = crate::component::theme::wallpaper_opacity(cx) * 100.;
        let wallpaper_opacity = cx.new(move |_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(wallpaper_opacity_value)
        });
        let window_opacity_value = crate::component::theme::window_opacity(cx) * 100.;
        let window_opacity = cx.new(move |_| {
            SliderState::new()
                .min(20.)
                .max(100.)
                .step(1.)
                .default_value(window_opacity_value)
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
        cx.subscribe(&font_color_picker, |_, _, event: &ColorPickerEvent, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                crate::component::theme::set_font_color(*color, cx);
            }
        })
        .detach();
        cx.subscribe(
            &background_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    crate::component::theme::set_background_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(&hover_color_picker, |_, _, event: &ColorPickerEvent, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                theme::set_hover_color(*color, cx);
            }
        })
        .detach();
        cx.subscribe(
            &selected_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    theme::set_selected_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(&wallpaper_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::set_wallpaper_opacity(value.start() / 100., cx);
            }
        })
        .detach();
        cx.subscribe(&window_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::set_window_opacity(value.start() / 100., cx);
            }
        })
        .detach();
        Self {
            color_picker,
            font_color_picker,
            background_color_picker,
            hover_color_picker,
            selected_color_picker,
            wallpaper_opacity,
            window_opacity,
            wallpaper_error: None,
            active_section: SettingsSection::Theme,
        }
    }

    pub(super) fn sync_color_pickers(&self, window: &mut Window, cx: &mut Context<Self>) {
        let styles = theme::styles(cx);
        Self::sync_picker(
            &self.font_color_picker,
            theme::font_color(cx).unwrap_or(styles.foreground),
            window,
            cx,
        );
        Self::sync_picker(
            &self.background_color_picker,
            theme::background_color(cx).unwrap_or(styles.background),
            window,
            cx,
        );
        Self::sync_picker(
            &self.hover_color_picker,
            theme::hover_color(cx).unwrap_or(styles.hover),
            window,
            cx,
        );
        Self::sync_picker(
            &self.selected_color_picker,
            theme::selected_color(cx).unwrap_or(styles.selected),
            window,
            cx,
        );
    }

    fn sync_picker(
        picker: &Entity<ColorPickerState>,
        color: Hsla,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if picker.read(cx).value() != Some(color) {
            picker.update(cx, |picker, cx| picker.set_value(color, window, cx));
        }
    }
}

impl Render for SettingsOperationWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_color_pickers(window, cx);
        self.render_view(window, cx)
    }
}
