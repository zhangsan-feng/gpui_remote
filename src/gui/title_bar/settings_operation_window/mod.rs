use gpui::*;
use gpui_component::{
    ActiveTheme,
    color_picker::{ColorPickerEvent, ColorPickerState},
    input::InputState,
    slider::{SliderEvent, SliderState},
};

use crate::{component::theme, infrastructure::agent_mcp};

mod core;
mod external;
mod internal;
mod ui;

pub(crate) use external::open_settings_window;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SettingsSection {
    Theme,
    Wallpaper,
    Mcp,
}

pub struct SettingsOperationWindow {
    color_picker: Entity<ColorPickerState>,
    font_color_picker: Entity<ColorPickerState>,
    background_color_picker: Entity<ColorPickerState>,
    hover_color_picker: Entity<ColorPickerState>,
    selected_color_picker: Entity<ColorPickerState>,
    wallpaper_opacity: Entity<SliderState>,
    wallpaper_error: Option<String>,
    mcp_enabled: bool,
    mcp_host: Entity<InputState>,
    mcp_port: Entity<InputState>,
    mcp_token: String,
    mcp_error: Option<String>,
    active_section: SettingsSection,
}

impl SettingsOperationWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx)
                .default_value(crate::component::theme::CustomerUiColor::custom_accent(cx))
        });
        let font_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                crate::component::theme::CustomerUiColor::font_color(cx)
                    .unwrap_or(cx.theme().foreground),
            )
        });
        let background_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                crate::component::theme::CustomerUiColor::background_color(cx)
                    .unwrap_or(cx.theme().background),
            )
        });
        let hover_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                theme::CustomerUiColor::hover_color(cx)
                    .unwrap_or(theme::CustomerUiTheme::colors(cx).hover_background),
            )
        });
        let selected_color_picker = cx.new(|cx| {
            ColorPickerState::new(window, cx).default_value(
                theme::CustomerUiColor::selected_color(cx)
                    .unwrap_or(theme::CustomerUiTheme::colors(cx).select_background),
            )
        });
        let wallpaper_opacity_value =
            crate::component::theme::CustomerUiColor::wallpaper_opacity(cx) * 100.;
        let wallpaper_opacity = cx.new(move |_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(wallpaper_opacity_value)
        });
        let mcp_settings = agent_mcp::settings();
        cx.subscribe(
            &color_picker,
            |_, _, event: &ColorPickerEvent, cx| match event {
                ColorPickerEvent::Change(Some(color)) => {
                    crate::component::theme::CustomerTheme::select_custom(*color, cx);
                }
                ColorPickerEvent::Change(None) => {}
            },
        )
        .detach();
        cx.subscribe(&font_color_picker, |_, _, event: &ColorPickerEvent, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                crate::component::theme::CustomerUiColor::set_font_color(*color, cx);
            }
        })
        .detach();
        cx.subscribe(
            &background_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    crate::component::theme::CustomerUiColor::set_background_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(&hover_color_picker, |_, _, event: &ColorPickerEvent, cx| {
            if let ColorPickerEvent::Change(Some(color)) = event {
                theme::CustomerUiColor::set_hover_color(*color, cx);
            }
        })
        .detach();
        cx.subscribe(
            &selected_color_picker,
            |_, _, event: &ColorPickerEvent, cx| {
                if let ColorPickerEvent::Change(Some(color)) = event {
                    theme::CustomerUiColor::set_selected_color(*color, cx);
                }
            },
        )
        .detach();
        cx.subscribe(&wallpaper_opacity, |_, _, event: &SliderEvent, cx| {
            if let SliderEvent::Change(value) = event {
                crate::component::theme::CustomerUiColor::set_wallpaper_opacity(
                    value.start() / 100.,
                    cx,
                );
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
            wallpaper_error: None,
            mcp_enabled: mcp_settings.enabled,
            mcp_host: Self::input_with_value(mcp_settings.host, "监听地址", window, cx),
            mcp_port: Self::input_with_value(mcp_settings.port.to_string(), "监听端口", window, cx),
            mcp_token: mcp_settings.token,
            mcp_error: None,
            active_section: SettingsSection::Theme,
        }
    }

    fn input_with_value(
        value: String,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value)
        })
    }

    pub(super) fn sync_color_pickers(&self, window: &mut Window, cx: &mut Context<Self>) {
        let colors = theme::CustomerUiTheme::colors(cx);
        Self::sync_picker(
            &self.font_color_picker,
            theme::CustomerUiColor::font_color(cx).unwrap_or(colors.text_color),
            window,
            cx,
        );
        Self::sync_picker(
            &self.background_color_picker,
            theme::CustomerUiColor::background_color(cx).unwrap_or(colors.background),
            window,
            cx,
        );
        Self::sync_picker(
            &self.hover_color_picker,
            theme::CustomerUiColor::hover_color(cx).unwrap_or(colors.hover_background),
            window,
            cx,
        );
        Self::sync_picker(
            &self.selected_color_picker,
            theme::CustomerUiColor::selected_color(cx).unwrap_or(colors.select_background),
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
