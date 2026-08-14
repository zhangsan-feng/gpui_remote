use std::path::PathBuf;

use gpui::{App, Hsla};
use gpui_component::ActiveTheme;

use crate::component::color::rgb_to_u32;

use super::{
    AppTheme, CustomerUiThemeState, MIN_SELECTION_LIGHTNESS_CONTRAST, SELECTION_LIGHTNESS_OFFSET,
};

#[derive(Clone)]
pub struct GuiColor {
    pub background: Hsla,
    pub title_bar_background: Hsla,
    pub sidebar_background: Hsla,
    pub workspace_background: Hsla,
    pub workspace_select_color: Hsla,
    pub workspace_text_color: Hsla,
    pub select_background: Hsla,
    pub border_color: Hsla,
    pub hover_background: Hsla,
    pub text_color: Hsla,
    pub background_image: Option<PathBuf>,
    pub image_opacity: f32,
}

pub(super) fn build(cx: &App) -> GuiColor {
    let state = cx.global::<CustomerUiThemeState>();
    let theme = cx.theme();
    let wallpaper = has_wallpaper(cx);
    let workspace_base = if state.theme == AppTheme::Wisteria {
        rgb_to_u32(248, 247, 250).into()
    } else {
        theme.background
    };
    let workspace_background = if wallpaper {
        Hsla::transparent_black()
    } else {
        workspace_base
    };
    let workspace_text_color = state
        .colors
        .font
        .unwrap_or_else(|| default_foreground(workspace_base.l < 0.5));
    let workspace_select_color =
        if state.theme == AppTheme::Wisteria && state.colors.selected.is_none() {
            rgb_to_u32(224, 224, 224).into()
        } else {
            theme.selection
        };

    GuiColor {
        background: theme.background,
        title_bar_background: if wallpaper {
            theme.title_bar.opacity(0.15)
        } else {
            theme.title_bar
        },
        sidebar_background: if wallpaper {
            theme.sidebar.opacity(0.15)
        } else {
            theme.sidebar
        },
        workspace_background,
        workspace_select_color,
        workspace_text_color,
        select_background: theme.selection,
        border_color: theme.border.opacity(if wallpaper { 0.62 } else { 0.9 }),
        hover_background: theme.list_hover,
        text_color: theme.foreground,
        background_image: state.visual.wallpaper.clone(),
        image_opacity: state.visual.wallpaper_opacity,
    }
}

pub(super) fn has_wallpaper(cx: &App) -> bool {
    let visual = &cx.global::<CustomerUiThemeState>().visual;
    visual.wallpaper.is_some() && visual.wallpaper_opacity > 0.
}

pub(super) fn terminal_selection_background(cx: &App) -> Hsla {
    let colors = build(cx);
    let selected = cx
        .global::<CustomerUiThemeState>()
        .colors
        .selected
        .unwrap_or(colors.select_background);
    if (selected.l - colors.workspace_background.l).abs() >= MIN_SELECTION_LIGHTNESS_CONTRAST {
        selected
    } else {
        contrasting_selection(colors.workspace_background, selected)
    }
}

pub(super) fn terminal_selection_foreground(cx: &App) -> Hsla {
    let selection = terminal_selection_background(cx);
    default_foreground(selection.l < 0.5)
}

pub(super) fn default_foreground(dark: bool) -> Hsla {
    if dark {
        rgb_to_u32(245, 243, 242).into()
    } else {
        rgb_to_u32(0, 0, 0).into()
    }
}

fn contrasting_selection(background: Hsla, mut selected: Hsla) -> Hsla {
    selected.l = if background.l >= 0.5 {
        (background.l - SELECTION_LIGHTNESS_OFFSET).clamp(0.08, 0.92)
    } else {
        (background.l + SELECTION_LIGHTNESS_OFFSET).clamp(0.08, 0.92)
    };
    selected.a = 1.;
    selected
}
