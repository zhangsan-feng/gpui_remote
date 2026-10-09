use std::path::PathBuf;

use gpui_kit::component::ActiveTheme;
use gpui_kit::{App, Hsla, WindowBackgroundAppearance};

use super::{AppTheme, ChangeComponentThemeColor, CustomerUiThemeState, ThemePreview, core, ui};

pub struct CustomerTheme;

impl CustomerTheme {
    pub fn active(cx: &App) -> AppTheme {
        cx.global::<CustomerUiThemeState>().theme
    }

    pub fn preview(theme: AppTheme) -> ThemePreview {
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
}

pub struct CustomerUiTheme;

impl CustomerUiTheme {
    pub fn apply(cx: &mut App) {
        ChangeComponentThemeColor::apply(cx);
    }

    pub fn colors(cx: &App) -> super::GuiColor {
        ui::build(cx)
    }

    pub fn panel_background(cx: &App) -> Hsla {
        ui::build(cx).title_bar_background
    }

    pub fn border_color(cx: &App) -> Hsla {
        ui::build(cx).border_color
    }

    pub fn tab_background(cx: &App) -> Hsla {
        if ui::has_wallpaper(cx) {
            Hsla::transparent_black()
        } else {
            ui::build(cx).sidebar_background
        }
    }

    pub fn title_background(cx: &App) -> Hsla {
        ui::build(cx).title_bar_background
    }

    pub fn sidebar_background(cx: &App) -> Hsla {
        ui::build(cx).sidebar_background
    }

    pub fn workspace_background(cx: &App) -> Hsla {
        ui::build(cx).workspace_background
    }

    pub fn terminal_selection_background(cx: &App) -> Hsla {
        ui::terminal_selection_background(cx)
    }

    pub fn terminal_selection_foreground(cx: &App) -> Hsla {
        ui::terminal_selection_foreground(cx)
    }

    pub fn terminal_cursor_color(cx: &App) -> Hsla {
        let colors = ui::build(cx);
        let accent = cx.theme().primary;
        if (accent.l - colors.workspace_background.l).abs() >= 0.3 {
            accent
        } else {
            colors.workspace_text_color
        }
    }

    pub fn window_background_appearance(_: &App) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Transparent
    }
}

impl ChangeComponentThemeColor {
    pub fn apply(cx: &mut App) {
        core::apply_theme(cx);
    }
}
