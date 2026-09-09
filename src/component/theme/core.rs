use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui_kit::component::{Colorize, Theme, ThemeMode};
use gpui_kit::{App, Hsla, Rgba};

use crate::{
    component::color::rgb_to_u32,
    global_state::{GlobalEvent, read_global_state},
};

use super::{
    AppTheme, ColorOverrides, CustomerUiThemeState, HOVER_LIGHTNESS_OFFSET,
    MIN_HOVER_SELECTION_CONTRAST, REGION_BACKGROUND_OFFSET, SETTINGS_PATH, StoredColors,
    ThemePreview, ThemeSettings, VisualSettings,
};

#[derive(Clone, Copy)]
pub(super) struct ThemePalette {
    pub(super) accent: Rgba,
    pub(super) accent_light: Rgba,
    pub(super) accent_dark: Rgba,
    pub(super) soft: Rgba,
    pub(super) background: Rgba,
    pub(super) border: Rgba,
    pub(super) foreground: Rgba,
    pub(super) muted_foreground: Rgba,
}

impl AppTheme {
    pub(super) fn palette(self) -> ThemePalette {
        match self {
            Self::RoseBerry => palette_from_tokens(
                (255, 58, 131),
                (251, 133, 177),
                (219, 0, 81),
                (255, 204, 223),
                (255, 204, 223),
                (255, 135, 180),
                (138, 0, 51),
                (219, 0, 81),
            ),
            Self::DefaultTheme => palette_from_tokens(
                (17, 17, 17),
                (115, 115, 115),
                (0, 0, 0),
                (229, 229, 229),
                (255, 255, 255),
                (163, 163, 163),
                (17, 17, 17),
                (115, 115, 115),
            ),
            Self::LightBlue => palette_from_tokens(
                (117, 179, 255),
                (169, 207, 252),
                (5, 118, 255),
                (214, 231, 252),
                (214, 231, 252),
                (169, 207, 252),
                (0, 74, 164),
                (5, 118, 255),
            ),
            Self::LightOrange => palette_from_tokens(
                (255, 205, 117),
                (252, 222, 169),
                (255, 164, 5),
                (252, 238, 214),
                (252, 238, 214),
                (252, 222, 169),
                (164, 104, 0),
                (255, 164, 5),
            ),
            Self::LightPurple => palette_from_tokens(
                (255, 54, 243),
                (251, 130, 244),
                (216, 0, 203),
                (250, 195, 247),
                (250, 195, 247),
                (251, 130, 244),
                (136, 0, 128),
                (216, 0, 203),
            ),
            Self::LightPink => palette_from_tokens(
                (255, 64, 201),
                (251, 136, 219),
                (223, 0, 160),
                (251, 198, 236),
                (251, 198, 236),
                (251, 136, 219),
                (140, 0, 101),
                (223, 0, 160),
            ),
            Self::Custom => unreachable!("自定义主题需要使用自定义调色板"),
        }
    }
}

pub(super) fn initialize(cx: &mut App) {
    let settings = load_settings();
    let colors = ColorOverrides {
        accent: parse_color(&settings.colors.accent).unwrap_or_else(default_custom_color),
        font: parse_optional_color(settings.colors.font),
        background: parse_optional_color(settings.colors.background),
        hover: parse_optional_color(settings.colors.hover),
        selected: parse_optional_color(settings.colors.selected),
    };
    let visual = VisualSettings {
        wallpaper: settings.wallpaper.map(PathBuf::from),
        wallpaper_opacity: settings.wallpaper_opacity.clamp(0., 1.),
    };

    cx.set_global(CustomerUiThemeState {
        theme: settings.theme,
        colors,
        visual,
    });
}

pub(super) fn preview(theme: AppTheme) -> ThemePreview {
    let palette = if theme == AppTheme::Custom {
        custom_palette(default_custom_color())
    } else {
        theme.palette()
    };
    let accent: Hsla = palette.accent.into();
    let background: Hsla = palette.background.into();
    ThemePreview {
        accent,
        background,
        hover: derive_hover(palette.soft.into(), derive_selected(background, accent)),
    }
}

pub(super) fn apply_theme(cx: &mut App) {
    let (app_theme, palette, colors) = {
        let state = cx.global::<CustomerUiThemeState>();
        let palette = if state.theme == AppTheme::Custom {
            custom_palette(state.colors.accent)
        } else {
            state.theme.palette()
        };
        let colors = if state.theme == AppTheme::Custom {
            state.colors
        } else {
            ColorOverrides {
                accent: palette.accent.into(),
                font: state.colors.font,
                background: state.colors.background,
                hover: state.colors.hover,
                selected: state.colors.selected,
            }
        };
        (state.theme, palette, colors)
    };

    let accent: Hsla = palette.accent.into();
    let accent_light: Hsla = palette.accent_light.into();
    let accent_dark: Hsla = palette.accent_dark.into();
    let soft: Hsla = palette.soft.into();
    let background = colors
        .background
        .unwrap_or_else(|| palette.background.into());
    let surface = derive_surface(background);
    let sidebar = derive_sidebar(background);
    let border = colors
        .background
        .map(derive_border)
        .unwrap_or_else(|| palette.border.into());
    let dark = background.l < 0.5;
    let foreground = colors.font.unwrap_or_else(|| {
        if app_theme == AppTheme::Custom || colors.background.is_some() {
            default_foreground(dark)
        } else {
            palette.foreground.into()
        }
    });
    let muted_foreground_color =
        if app_theme == AppTheme::Custom || colors.font.is_some() || colors.background.is_some() {
            muted_foreground(dark)
        } else {
            palette.muted_foreground.into()
        };
    let selected = colors
        .selected
        .unwrap_or_else(|| derive_selected(background, accent));
    let hover = colors.hover.unwrap_or_else(|| derive_hover(soft, selected));
    let primary_hover = with_alpha(accent_light, 0.9);
    let primary_active = with_alpha(accent_dark, 0.92);
    let primary_foreground = default_foreground(accent.l < 0.5);

    let theme = Theme::global_mut(cx);
    theme.mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    theme.background = background;
    theme.foreground = foreground;
    theme.muted_foreground = muted_foreground_color;
    theme.muted = surface;
    theme.secondary = surface;
    theme.secondary_foreground = foreground;
    theme.secondary_hover = hover;
    theme.secondary_active = selected;
    theme.border = border;
    theme.input = border;
    theme.popover = background;
    theme.popover_foreground = foreground;
    theme.title_bar = surface;
    theme.title_bar_border = border;
    theme.sidebar = sidebar;
    theme.sidebar_border = border;
    theme.sidebar_foreground = foreground;
    theme.tab = surface;
    theme.tab_bar = surface;
    theme.tab_bar_segmented = sidebar;
    theme.tab_foreground = foreground;
    theme.tab_active = background;
    theme.tab_active_foreground = foreground;
    theme.colors.list = background;
    theme.list_head = surface;
    theme.table = background;
    theme.table_head = surface;
    theme.table_row_border = border;
    theme.group_box = surface;
    theme.tiles = background;
    theme.primary = accent;
    theme.primary_hover = primary_hover;
    theme.primary_active = primary_active;
    theme.primary_foreground = primary_foreground;
    theme.button = surface;
    theme.button_hover = hover;
    theme.button_active = selected;
    theme.button_foreground = foreground;
    theme.button_primary = accent;
    theme.button_primary_hover = primary_hover;
    theme.button_primary_active = primary_active;
    theme.button_primary_foreground = primary_foreground;
    theme.button_secondary = surface;
    theme.button_secondary_hover = hover;
    theme.button_secondary_active = selected;
    theme.button_secondary_foreground = foreground;
    theme.accent = soft;
    theme.accent_foreground = default_foreground(soft.l < 0.5);
    theme.ring = with_alpha(accent, 0.45);
    theme.selection = selected;
    theme.link = accent;
    theme.link_hover = with_alpha(accent_light, 0.9);
    theme.link_active = with_alpha(accent_dark, 0.9);
    theme.sidebar_primary = accent;
    theme.sidebar_primary_foreground = default_foreground(accent.l < 0.5);
    theme.sidebar_accent = soft;
    theme.sidebar_accent_foreground = default_foreground(soft.l < 0.5);
    theme.list_active = selected;
    theme.list_active_border = with_alpha(accent, 0.35);
    theme.list_hover = hover;
    theme.caret = foreground;
    theme.progress_bar = accent;
    theme.slider_thumb = accent;
    let colors = theme.colors;
    theme.tokens = (&colors).into();
}

pub(super) fn refresh(cx: &mut App, reapply_theme: bool) {
    if reapply_theme {
        apply_theme(cx);
    }
    persist(cx);
    cx.refresh_windows();
}

fn palette_from_tokens(
    accent: (u8, u8, u8),
    accent_light: (u8, u8, u8),
    accent_dark: (u8, u8, u8),
    soft: (u8, u8, u8),
    background: (u8, u8, u8),
    border: (u8, u8, u8),
    foreground: (u8, u8, u8),
    muted_foreground: (u8, u8, u8),
) -> ThemePalette {
    ThemePalette {
        accent: rgb_to_u32(accent.0, accent.1, accent.2),
        accent_light: rgb_to_u32(accent_light.0, accent_light.1, accent_light.2),
        accent_dark: rgb_to_u32(accent_dark.0, accent_dark.1, accent_dark.2),
        soft: rgb_to_u32(soft.0, soft.1, soft.2),
        background: rgb_to_u32(background.0, background.1, background.2),
        border: rgb_to_u32(border.0, border.1, border.2),
        foreground: rgb_to_u32(foreground.0, foreground.1, foreground.2),
        muted_foreground: rgb_to_u32(muted_foreground.0, muted_foreground.1, muted_foreground.2),
    }
}

fn custom_palette(accent: Hsla) -> ThemePalette {
    let dark = accent.l < 0.5;
    let background = accent;
    let accent_light = shift_background(background, 0.1, 0.9);
    let accent_dark = shift_background(background, -0.1, 1.0);
    let soft = shift_background(background, if dark { 0.11 } else { -0.08 }, 0.88);
    ThemePalette {
        accent: accent.into(),
        accent_light: accent_light.into(),
        accent_dark: accent_dark.into(),
        soft: soft.into(),
        background: background.into(),
        border: derive_border(background).into(),
        foreground: default_foreground(dark).into(),
        muted_foreground: muted_foreground(dark).into(),
    }
}

pub(super) fn default_foreground(dark: bool) -> Hsla {
    if dark {
        rgb_to_u32(245, 243, 242).into()
    } else {
        rgb_to_u32(17, 17, 17).into()
    }
}

fn derive_surface(background: Hsla) -> Hsla {
    shift_background(background, region_lightness_offset(background), 0.96)
}

fn derive_sidebar(background: Hsla) -> Hsla {
    shift_background(background, region_lightness_offset(background), 0.88)
}

fn region_lightness_offset(background: Hsla) -> f32 {
    if background.l < 0.5 {
        REGION_BACKGROUND_OFFSET
    } else {
        -REGION_BACKGROUND_OFFSET
    }
}

fn derive_border(background: Hsla) -> Hsla {
    shift_background(
        background,
        if background.l < 0.5 { 0.12 } else { -0.12 },
        0.52,
    )
}

fn derive_selected(background: Hsla, accent: Hsla) -> Hsla {
    if background.l < 0.5 {
        shift_background(background, 0.18, 0.92)
    } else if accent.l > 0.7 {
        accent
    } else {
        let mut selected = accent;
        selected.l = 0.93;
        selected.s = (selected.s * 0.35).clamp(0., 1.);
        selected.a = 1.;
        selected
    }
}

fn derive_hover(soft: Hsla, selected: Hsla) -> Hsla {
    let mut hover = shift_background(soft, -HOVER_LIGHTNESS_OFFSET, 0.92);
    if (hover.l - selected.l).abs() < MIN_HOVER_SELECTION_CONTRAST {
        hover.l = (selected.l - MIN_HOVER_SELECTION_CONTRAST).clamp(0.02, 0.98);
    }
    hover
}

fn shift_background(mut color: Hsla, lightness_delta: f32, saturation_scale: f32) -> Hsla {
    color.l = (color.l + lightness_delta).clamp(0.02, 0.98);
    color.s = (color.s * saturation_scale).clamp(0., 1.);
    color.a = 1.;
    color
}

fn muted_foreground(dark: bool) -> Hsla {
    if dark {
        rgb_to_u32(184, 176, 172).into()
    } else {
        rgb_to_u32(139, 109, 104).into()
    }
}

fn with_alpha(mut color: Hsla, alpha: f32) -> Hsla {
    color.a = alpha;
    color
}

fn load_settings() -> ThemeSettings {
    fs::read(SETTINGS_PATH)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn persist(cx: &mut App) {
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
        }
    };

    if let Err(error) = save_settings(&settings) {
        log::error!("保存主题设置失败: {error}");
    }
    read_global_state(cx).update(cx, |_, cx| cx.emit(GlobalEvent::ThemeColorChanged));
}

fn save_settings(settings: &ThemeSettings) -> std::io::Result<()> {
    if let Some(parent) = Path::new(SETTINGS_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(SETTINGS_PATH, bytes)
}

fn parse_optional_color(value: Option<String>) -> Option<Hsla> {
    value.and_then(|value| parse_color(&value))
}

fn parse_color(value: &str) -> Option<Hsla> {
    Hsla::parse_hex(value).ok()
}

pub(super) fn default_accent() -> String {
    "#FF3A83".to_owned()
}

fn default_custom_color() -> Hsla {
    Hsla::parse_hex(&default_accent()).expect("默认自定义主题颜色必须有效")
}
