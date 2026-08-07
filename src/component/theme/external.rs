use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{App, Hsla, WindowBackgroundAppearance};
use gpui_component::ActiveTheme;

use super::{
    AppTheme, ChangeComponentThemeColor, CustomerUiThemeState, ThemePreview, WALLPAPER_DIRECTORY,
    core, ui,
};

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
        if theme != AppTheme::Custom {
            state.colors.font = None;
            state.colors.background = None;
            state.colors.hover = None;
            state.colors.selected = None;
        }
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

    pub fn set_wallpaper(source: &Path, cx: &mut App) -> Result<(), String> {
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| "无法识别图片格式".to_owned())?;
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            return Err("仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned());
        }

        let directory = Path::new(WALLPAPER_DIRECTORY);
        fs::create_dir_all(directory).map_err(|error| error.to_string())?;
        let target = directory.join(format!("wallpaper.{extension}"));
        fs::copy(source, &target).map_err(|error| error.to_string())?;
        cx.global_mut::<CustomerUiThemeState>().visual.wallpaper = Some(target);
        core::refresh(cx, false);
        Ok(())
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
        ui::build(cx)
            .background
            .opacity(if ui::has_wallpaper(cx) { 0.15 } else { 1. })
    }

    pub fn border_color(cx: &App) -> Hsla {
        ui::build(cx).border_color
    }

    pub fn tab_background(cx: &App) -> Hsla {
        if ui::has_wallpaper(cx) {
            Hsla::transparent_black()
        } else {
            cx.theme().tab_bar
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

    pub fn window_background_appearance(_: &App) -> WindowBackgroundAppearance {
        WindowBackgroundAppearance::Transparent
    }
}

impl ChangeComponentThemeColor {
    pub fn apply(cx: &mut App) {
        core::apply_theme(cx);
    }
}
