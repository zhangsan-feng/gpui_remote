use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{App, Hsla, Rgba};
use gpui_component::{Colorize, Theme, ThemeMode};

use crate::{
    component::color::rgb_to_u32,
    global_state::{GlobalEvent, read_global_state},
};

use super::{
    AppTheme, ColorOverrides, CustomerUiThemeState, SETTINGS_PATH, StoredColors, ThemePreview,
    ThemeSettings, VisualSettings,
};

#[derive(Clone, Copy)]
pub(super) struct ThemePalette {
    pub(super) accent: Rgba,
    pub(super) accent_light: Rgba,
    pub(super) accent_dark: Rgba,
    pub(super) soft: Rgba,
    pub(super) background: Rgba,
    pub(super) surface: Rgba,
    pub(super) sidebar: Rgba,
    pub(super) border: Rgba,
}

impl AppTheme {
    pub(super) fn palette(self) -> ThemePalette {
        match self {
            Self::Wisteria => material_light_palette(),
            Self::SeaSalt => material_palette(
                (3, 169, 244),
                (41, 182, 246),
                (2, 136, 209),
                (225, 245, 254),
                (179, 229, 252),
                (129, 212, 250),
            ),
            Self::Moss => material_palette(
                (76, 175, 80),
                (102, 187, 106),
                (56, 142, 60),
                (232, 245, 233),
                (200, 230, 201),
                (165, 214, 167),
            ),
            Self::WarmSand => material_palette(
                (255, 193, 7),
                (255, 202, 40),
                (255, 160, 0),
                (255, 248, 225),
                (255, 236, 179),
                (255, 224, 130),
            ),
            Self::MaterialRed => material_palette(
                (244, 67, 54),
                (239, 83, 80),
                (211, 47, 47),
                (255, 235, 238),
                (255, 205, 210),
                (239, 154, 154),
            ),
            Self::MaterialPink => material_palette(
                (233, 30, 99),
                (236, 64, 122),
                (194, 24, 91),
                (252, 228, 236),
                (248, 187, 208),
                (244, 143, 177),
            ),
            Self::MaterialDeepOrange => material_palette(
                (255, 87, 34),
                (255, 112, 67),
                (230, 74, 25),
                (251, 233, 231),
                (255, 204, 188),
                (255, 171, 145),
            ),
            Self::MaterialOrange => material_palette(
                (255, 152, 0),
                (255, 167, 38),
                (245, 124, 0),
                (255, 243, 224),
                (255, 224, 178),
                (255, 204, 128),
            ),
            Self::MaterialAmber => material_palette(
                (255, 193, 7),
                (255, 202, 40),
                (255, 160, 0),
                (255, 248, 225),
                (255, 236, 179),
                (255, 224, 130),
            ),
            Self::MaterialBrown => material_palette(
                (121, 85, 72),
                (141, 110, 99),
                (93, 64, 55),
                (239, 235, 233),
                (215, 204, 200),
                (188, 170, 164),
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
    ThemePreview {
        accent: palette.accent.into(),
        background: palette.background.into(),
        hover: palette.soft.into(),
    }
}

pub(super) fn apply_theme(cx: &mut App) {
    let (palette, colors) = {
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
        (palette, colors)
    };

    let accent: Hsla = palette.accent.into();
    let accent_light: Hsla = palette.accent_light.into();
    let accent_dark: Hsla = palette.accent_dark.into();
    let soft: Hsla = palette.soft.into();
    let background = colors
        .background
        .unwrap_or_else(|| palette.background.into());
    let surface = colors
        .background
        .map(derive_surface)
        .unwrap_or_else(|| palette.surface.into());
    let sidebar = colors
        .background
        .map(derive_sidebar)
        .unwrap_or_else(|| palette.sidebar.into());
    let border = colors
        .background
        .map(derive_border)
        .unwrap_or_else(|| palette.border.into());
    let dark = background.l < 0.5;
    let foreground = colors.font.unwrap_or_else(|| default_foreground(dark));
    let hover = colors.hover.unwrap_or(soft);
    let selected = colors
        .selected
        .unwrap_or_else(|| derive_selected(background, accent));

    let theme = Theme::global_mut(cx);
    theme.mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    theme.background = background;
    theme.foreground = foreground;
    theme.muted_foreground = muted_foreground(dark);
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
    theme.primary_hover = with_alpha(accent_light, 0.9);
    theme.primary_active = with_alpha(accent_dark, 0.92);
    theme.primary_foreground = default_foreground(accent.l < 0.5);
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

fn material_light_palette() -> ThemePalette {
    ThemePalette {
        accent: rgb_to_u32(156, 39, 176),
        accent_light: rgb_to_u32(186, 104, 200),
        accent_dark: rgb_to_u32(123, 31, 162),
        soft: rgb_to_u32(243, 229, 245),
        background: rgb_to_u32(245, 245, 245),
        surface: rgb_to_u32(250, 245, 251),
        sidebar: rgb_to_u32(255, 255, 255),
        border: rgb_to_u32(206, 147, 216),
    }
}

fn material_palette(
    accent: (u8, u8, u8),
    accent_light: (u8, u8, u8),
    accent_dark: (u8, u8, u8),
    soft: (u8, u8, u8),
    surface: (u8, u8, u8),
    border: (u8, u8, u8),
) -> ThemePalette {
    ThemePalette {
        accent: rgb_to_u32(accent.0, accent.1, accent.2),
        accent_light: rgb_to_u32(accent_light.0, accent_light.1, accent_light.2),
        accent_dark: rgb_to_u32(accent_dark.0, accent_dark.1, accent_dark.2),
        soft: rgb_to_u32(soft.0, soft.1, soft.2),
        background: rgb_to_u32(soft.0, soft.1, soft.2),
        surface: rgb_to_u32(surface.0, surface.1, surface.2),
        sidebar: rgb_to_u32(soft.0, soft.1, soft.2),
        border: rgb_to_u32(border.0, border.1, border.2),
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
        surface: derive_surface(background).into(),
        sidebar: derive_sidebar(background).into(),
        border: derive_border(background).into(),
    }
}

pub(super) fn default_foreground(dark: bool) -> Hsla {
    if dark {
        rgb_to_u32(245, 243, 242).into()
    } else {
        rgb_to_u32(0, 0, 0).into()
    }
}

fn derive_surface(background: Hsla) -> Hsla {
    shift_background(
        background,
        if background.l < 0.5 { 0.035 } else { -0.025 },
        0.82,
    )
}

fn derive_sidebar(background: Hsla) -> Hsla {
    shift_background(
        background,
        if background.l < 0.5 { 0.065 } else { -0.045 },
        0.68,
    )
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
        rgb_to_u32(107, 99, 120).into()
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
    "#212121".to_owned()
}

fn default_custom_color() -> Hsla {
    Hsla::parse_hex(&default_accent()).expect("默认自定义主题颜色必须有效")
}
