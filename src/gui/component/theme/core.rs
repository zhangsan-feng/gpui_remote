use std::path::PathBuf;

use gpui_kit::component::{Colorize, Theme, ThemeColor, ThemeMode};
use gpui_kit::{App, Hsla, rgb};

use crate::{
    domain::terminal::{MAX_TERMINAL_FONT_SIZE, MIN_TERMINAL_FONT_SIZE},
    event_bus::{EVENT_BUS, gui_event::GuiEvent},
};

use super::{
    AppTheme, ColorOverrides, CustomerUiThemeState, StoredColors, TerminalAppearance,
    ThemeSettings, VisualSettings,
};

pub(super) fn initialize(
    cx: &mut App,
    settings: ThemeSettings,
    persist_settings: tokio::sync::watch::Sender<ThemeSettings>,
    settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    log::debug!("主题初始化: theme={:?}", settings.theme);
    let colors = ColorOverrides {
        accent: parse_color(&settings.colors.accent).unwrap_or_else(|| rgb(0xff3a83).into()),
        font: parse_optional_color(settings.colors.font),
        background: parse_optional_color(settings.colors.background),
        hover: parse_optional_color(settings.colors.hover),
        selected: parse_optional_color(settings.colors.selected),
    };
    let visual = VisualSettings {
        wallpaper: settings.wallpaper.map(PathBuf::from),
        wallpaper_opacity: settings.wallpaper_opacity.clamp(0., 1.),
    };

    cx.set_global(TerminalAppearance {
        font_size: settings
            .terminal_font_size
            .clamp(MIN_TERMINAL_FONT_SIZE, MAX_TERMINAL_FONT_SIZE),
    });
    cx.set_global(CustomerUiThemeState {
        theme: settings.theme,
        colors,
        visual,
        persist_settings,
        settings_changed,
    });
}

pub(super) fn preview(preset: AppTheme) -> ThemeColor {
    let mut theme = *ThemeColor::light();
    apply_variables(&mut theme, preset, ColorOverrides::default());
    theme
}

pub(super) fn apply_theme(cx: &mut App) {
    let state = cx.global::<CustomerUiThemeState>();
    let (preset, overrides) = (state.theme, state.colors);
    let background = overrides
        .background
        .or_else(|| preset_background(preset, overrides.accent));
    let mode = if background.is_some_and(|color| color.l < 0.5) {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };

    // Reload the toolkit defaults so switching presets or clearing an override
    // cannot inherit values from the previously active theme.
    Theme::change(mode, None, cx);
    Theme::update(cx, |theme| {
        apply_variables(&mut theme.colors, preset, overrides)
    });
    let colors = &Theme::global(cx).colors;
    log::debug!(
        "theme_applied: preset={preset:?}, mode={mode:?}, background={}, sidebar={}, surface={}, foreground={}",
        colors.background.to_hex(),
        colors.sidebar.to_hex(),
        colors.group_box.to_hex(),
        colors.foreground.to_hex(),
    );
}

fn preset_accent(preset: AppTheme, custom: Hsla) -> Option<Hsla> {
    match preset {
        AppTheme::DefaultTheme => None,
        AppTheme::RoseBerry => Some(rgb(0xff3a83).into()),
        AppTheme::LightBlue => Some(rgb(0x75b3ff).into()),
        AppTheme::LightOrange => Some(rgb(0xffa74f).into()),
        AppTheme::LightPurple => Some(rgb(0xb982ff).into()),
        AppTheme::LightPink => Some(rgb(0xff90db).into()),
        AppTheme::Custom => Some(custom),
    }
}

fn preset_background(preset: AppTheme, custom: Hsla) -> Option<Hsla> {
    match preset {
        AppTheme::DefaultTheme => None,
        AppTheme::RoseBerry | AppTheme::LightPink => Some(rgb(0xffccdf).into()),
        AppTheme::LightBlue => Some(rgb(0xd6e8ff).into()),
        AppTheme::LightOrange => Some(rgb(0xffe1c2).into()),
        AppTheme::LightPurple => Some(rgb(0xecd8ff).into()),
        AppTheme::Custom => Some(custom),
    }
}

fn apply_variables(colors: &mut ThemeColor, preset: AppTheme, overrides: ColorOverrides) {
    if let Some(accent) = preset_accent(preset, overrides.accent) {
        let foreground = if accent.l < 0.5 {
            ThemeColor::dark()
        } else {
            ThemeColor::light()
        }
        .foreground;
        colors.primary = accent;
        colors.primary_foreground = foreground;
        colors.primary_hover = tint(accent, foreground, 0.08);
        colors.primary_active = tint(accent, foreground, 0.16);
        colors.button_primary = colors.primary;
        colors.button_primary_foreground = colors.primary_foreground;
        colors.button_primary_hover = colors.primary_hover;
        colors.button_primary_active = colors.primary_active;
        colors.sidebar_primary = accent;
        colors.sidebar_primary_foreground = foreground;
        colors.link = tint(accent, colors.foreground, 0.3);
        colors.link_hover = tint(colors.link, colors.foreground, 0.12);
        colors.link_active = colors.link_hover;
        colors.ring = accent.opacity(0.55);
        colors.progress_bar = accent;
        colors.slider_thumb = accent;
    }

    if let Some(background) = overrides
        .background
        .or_else(|| preset_background(preset, overrides.accent))
    {
        let foreground = colors.foreground;
        let surface = tint(background, foreground, 0.035);
        let sidebar = tint(background, foreground, 0.055);
        let border = tint(background, foreground, 0.17);
        colors.background = background;
        colors.popover = background;
        colors.list = background;
        colors.table = background;
        colors.tab_active = background;
        colors.muted = surface;
        colors.secondary = surface;
        colors.group_box = surface;
        colors.button = surface;
        colors.button_secondary = surface;
        colors.sidebar = sidebar;
        colors.title_bar = sidebar;
        colors.status_bar = sidebar;
        colors.tab = sidebar;
        colors.tab_bar = sidebar;
        colors.tab_bar_segmented = sidebar;
        colors.list_head = surface;
        colors.table_head = surface;
        colors.border = border;
        colors.input = border;
        colors.sidebar_border = border;
        colors.title_bar_border = border;
        colors.status_bar_border = border;
        colors.table_row_border = border;
    }

    if let Some(foreground) = overrides.font {
        colors.foreground = foreground;
        colors.popover_foreground = foreground;
        colors.sidebar_foreground = foreground;
        colors.tab_foreground = foreground;
        colors.tab_active_foreground = foreground;
        colors.button_foreground = foreground;
        colors.button_secondary_foreground = foreground;
        colors.secondary_foreground = foreground;
        colors.caret = foreground;
    }

    if let Some(hover) = overrides.hover {
        colors.list_hover = hover;
        colors.table_hover = hover;
        colors.button_hover = hover;
        colors.button_secondary_hover = hover;
        colors.secondary_hover = hover;
    }

    if let Some(selected) = overrides.selected {
        colors.selection = selected;
        colors.tab_active = selected;
        colors.list_active = selected;
        colors.table_active = selected;
        colors.sidebar_accent = selected;
        colors.button_active = selected;
        colors.button_secondary_active = selected;
        colors.secondary_active = selected;
    }
}

fn tint(base: Hsla, overlay: Hsla, overlay_weight: f32) -> Hsla {
    // GPUI Kit weights the receiver, so the overlay takes the first operand.
    overlay.mix_oklab(base, overlay_weight)
}

pub(super) fn refresh(cx: &mut App, reapply_theme: bool) {
    if reapply_theme {
        apply_theme(cx);
    } else {
        cx.refresh_windows();
    }
    persist(cx);
    EVENT_BUS.gui_event.broadcast(GuiEvent::ThemeColorChanged);
}

pub(super) fn persist(cx: &App) {
    let settings = {
        let state = cx.global::<CustomerUiThemeState>();
        ThemeSettings {
            theme: state.theme,
            colors: StoredColors {
                accent: state.colors.accent.to_hex(),
                font: state.colors.font.map(|color| color.to_hex()),
                background: state.colors.background.map(|color| color.to_hex()),
                hover: state.colors.hover.map(|color| color.to_hex()),
                selected: state.colors.selected.map(|color| color.to_hex()),
            },
            wallpaper: state
                .visual
                .wallpaper
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            wallpaper_opacity: state.visual.wallpaper_opacity,
            terminal_font_size: TerminalAppearance::font_size(cx),
        }
    };

    let state = cx.global::<CustomerUiThemeState>();
    state
        .settings_changed
        .store(true, std::sync::atomic::Ordering::Release);
    state.persist_settings.send_replace(settings);
}

fn parse_optional_color(value: Option<String>) -> Option<Hsla> {
    value.and_then(|value| parse_color(&value))
}

fn parse_color(value: &str) -> Option<Hsla> {
    Hsla::parse_hex(value).ok()
}
