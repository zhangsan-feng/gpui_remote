use std::path::PathBuf;

use gpui_kit::component::ThemeColor;
use gpui_kit::{App, Hsla};

use crate::domain::terminal::{MAX_TERMINAL_FONT_SIZE, MIN_TERMINAL_FONT_SIZE};

use super::{AppTheme, CustomerUiThemeState, TerminalAppearance, core};

impl TerminalAppearance {
    pub(crate) fn font_size(cx: &App) -> u8 {
        cx.global::<Self>().font_size
    }

    pub(crate) fn set_font_size(size: u8, cx: &mut App) {
        let size = size.clamp(MIN_TERMINAL_FONT_SIZE, MAX_TERMINAL_FONT_SIZE);
        if Self::font_size(cx) != size {
            cx.global_mut::<Self>().font_size = size;
            core::persist(cx);
        }
    }
}

pub struct CustomerTheme;

impl CustomerTheme {
    pub fn active(cx: &App) -> AppTheme {
        cx.global::<CustomerUiThemeState>().theme
    }

    pub fn preview(theme: AppTheme) -> ThemeColor {
        core::preview(theme)
    }

    pub fn select(theme: AppTheme, cx: &mut App) {
        let state = cx.global_mut::<CustomerUiThemeState>();
        state.theme = theme;
        core::refresh(cx, true);
    }

    pub fn select_custom(accent: Hsla, cx: &mut App) {
        let state = cx.global_mut::<CustomerUiThemeState>();
        state.theme = AppTheme::Custom;
        state.colors.accent = accent;
        core::refresh(cx, true);
    }
}

pub struct CustomerUiColor;

impl CustomerUiColor {
    pub fn custom_accent(cx: &App) -> Hsla {
        cx.global::<CustomerUiThemeState>().colors.accent
    }

    pub fn font_color(cx: &App) -> Option<Hsla> {
        cx.global::<CustomerUiThemeState>().colors.font
    }

    pub fn background_color(cx: &App) -> Option<Hsla> {
        cx.global::<CustomerUiThemeState>().colors.background
    }

    pub fn hover_color(cx: &App) -> Option<Hsla> {
        cx.global::<CustomerUiThemeState>().colors.hover
    }

    pub fn selected_color(cx: &App) -> Option<Hsla> {
        cx.global::<CustomerUiThemeState>().colors.selected
    }

    pub fn wallpaper(cx: &App) -> Option<(PathBuf, f32)> {
        let visual = &cx.global::<CustomerUiThemeState>().visual;
        visual
            .wallpaper
            .clone()
            .map(|path| (path, visual.wallpaper_opacity))
    }

    pub fn wallpaper_opacity(cx: &App) -> f32 {
        cx.global::<CustomerUiThemeState>().visual.wallpaper_opacity
    }

    pub fn set_font_color(color: Hsla, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.font = Some(color);
        core::refresh(cx, true);
    }

    pub fn clear_font_color(cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.font = None;
        core::refresh(cx, true);
    }

    pub fn set_background_color(color: Hsla, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.background = Some(color);
        core::refresh(cx, true);
    }

    pub fn clear_background_color(cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.background = None;
        core::refresh(cx, true);
    }

    pub fn set_hover_color(color: Hsla, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.hover = Some(color);
        core::refresh(cx, true);
    }

    pub fn clear_hover_color(cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.hover = None;
        core::refresh(cx, true);
    }

    pub fn set_selected_color(color: Hsla, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.selected = Some(color);
        core::refresh(cx, true);
    }

    pub fn clear_selected_color(cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().colors.selected = None;
        core::refresh(cx, true);
    }

    pub fn set_wallpaper_opacity(opacity: f32, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>()
            .visual
            .wallpaper_opacity = opacity.clamp(0., 1.);
        core::refresh(cx, false);
    }

    pub fn set_wallpaper_path(path: PathBuf, cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().visual.wallpaper = Some(path);
        core::refresh(cx, false);
    }

    pub fn clear_wallpaper(cx: &mut App) {
        cx.global_mut::<CustomerUiThemeState>().visual.wallpaper = None;
        core::refresh(cx, false);
    }

    pub fn has_wallpaper(cx: &App) -> bool {
        let visual = &cx.global::<CustomerUiThemeState>().visual;
        visual.wallpaper.is_some() && visual.wallpaper_opacity > 0.
    }
}
