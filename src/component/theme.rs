use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{App, Global, Hsla, Rgba, WindowBackgroundAppearance};
use gpui_component::{ActiveTheme, Colorize, Theme, ThemeMode};
use serde::{Deserialize, Serialize};

use crate::component::color::rgb_to_u32;
use crate::global_state::{GlobalEvent, read_global_state};

const SETTINGS_PATH: &str = "data/theme.json";
const WALLPAPER_DIRECTORY: &str = "data/background";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    #[default]
    Wisteria,
    SeaSalt,
    Moss,
    WarmSand,
    MaterialRed,
    MaterialPink,
    MaterialDeepOrange,
    MaterialOrange,
    MaterialAmber,
    MaterialBrown,
    Custom,
}

#[derive(Clone, Copy)]
struct ThemePalette {
    accent: Rgba,
    soft: Rgba,
    background: Rgba,
    surface: Rgba,
    sidebar: Rgba,
    border: Rgba,
}

impl AppTheme {
    fn palette(self) -> ThemePalette {
        match self {
            Self::Wisteria => palette((124, 58, 237), (245, 245, 245), (255, 255, 255)),
            Self::SeaSalt => palette((2, 132, 199), (224, 242, 254), (248, 252, 254)),
            Self::Moss => palette((5, 150, 105), (209, 250, 229), (249, 253, 251)),
            Self::WarmSand => palette((217, 119, 6), (254, 243, 199), (255, 253, 248)),
            Self::MaterialRed => material_palette((244, 67, 54), (255, 235, 238)),
            Self::MaterialPink => material_palette((233, 30, 99), (252, 228, 236)),
            Self::MaterialDeepOrange => material_palette((255, 87, 34), (251, 233, 231)),
            Self::MaterialOrange => material_palette((255, 152, 0), (255, 243, 224)),
            Self::MaterialAmber => material_palette((255, 193, 7), (255, 248, 225)),
            Self::MaterialBrown => material_palette((121, 85, 72), (239, 235, 233)),
            Self::Custom => unreachable!("自定义主题需要使用自定义调色板"),
        }
    }
}

fn palette(accent: (u8, u8, u8), soft: (u8, u8, u8), background: (u8, u8, u8)) -> ThemePalette {
    ThemePalette {
        accent: rgb_to_u32(accent.0, accent.1, accent.2),
        soft: rgb_to_u32(soft.0, soft.1, soft.2),
        background: rgb_to_u32(background.0, background.1, background.2),
        surface: rgb_to_u32(
            background.0.saturating_sub(5),
            background.1.saturating_sub(5),
            background.2.saturating_sub(5),
        ),
        sidebar: rgb_to_u32(
            background.0.saturating_sub(7),
            background.1.saturating_sub(7),
            background.2.saturating_sub(7),
        ),
        border: rgb_to_u32(225, 225, 225),
    }
}

fn material_palette(accent: (u8, u8, u8), soft: (u8, u8, u8)) -> ThemePalette {
    ThemePalette {
        accent: rgb_to_u32(accent.0, accent.1, accent.2),
        soft: rgb_to_u32(soft.0, soft.1, soft.2),
        background: rgb_to_u32(255, 253, 251),
        surface: rgb_to_u32(253, 248, 245),
        sidebar: rgb_to_u32(249, 244, 241),
        border: rgb_to_u32(232, 220, 214),
    }
}

#[derive(Clone, Copy, Default)]
struct ColorOverrides {
    accent: Hsla,
    font: Option<Hsla>,
    background: Option<Hsla>,
    hover: Option<Hsla>,
    selected: Option<Hsla>,
}

struct VisualSettings {
    wallpaper: Option<PathBuf>,
    wallpaper_opacity: f32,
}

struct AppThemeState {
    theme: AppTheme,
    colors: ColorOverrides,
    visual: VisualSettings,
}

impl Global for AppThemeState {}

#[derive(Clone)]
pub struct CustomThemeStyles {
    pub background: Hsla,
    pub foreground: Hsla,
    pub hover: Hsla,
    pub selected: Hsla,
    pub window_background: Hsla,
    pub window_background_img: Option<PathBuf>,
}

#[derive(Clone, Copy)]
pub struct ThemePreview {
    pub accent: Hsla,
    pub background: Hsla,
    pub hover: Hsla,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct ThemeSettings {
    theme: AppTheme,
    colors: StoredColors,
    wallpaper: Option<String>,
    wallpaper_opacity: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
struct StoredColors {
    accent: String,
    font: Option<String>,
    background: Option<String>,
    hover: Option<String>,
    selected: Option<String>,
}

impl Default for StoredColors {
    fn default() -> Self {
        Self {
            accent: default_accent(),
            font: None,
            background: None,
            hover: None,
            selected: None,
        }
    }
}

pub fn init(cx: &mut App) {
    let settings = load_settings();
    let mut colors = ColorOverrides {
        accent: parse_color(&settings.colors.accent).unwrap_or_else(default_custom_color),
        font: parse_optional_color(settings.colors.font),
        background: parse_optional_color(settings.colors.background),
        hover: parse_optional_color(settings.colors.hover),
        selected: parse_optional_color(settings.colors.selected),
    };
    if settings.theme != AppTheme::Custom {
        colors.font = None;
        colors.background = None;
        colors.hover = None;
        colors.selected = None;
    }
    let visual = VisualSettings {
        wallpaper: settings.wallpaper.map(PathBuf::from),
        wallpaper_opacity: settings.wallpaper_opacity.clamp(0., 1.),
    };

    cx.set_global(AppThemeState {
        theme: settings.theme,
        colors,
        visual,
    });
    apply_theme(cx);
}

pub fn preview(theme: AppTheme) -> ThemePreview {
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

pub fn active(cx: &App) -> AppTheme {
    cx.global::<AppThemeState>().theme
}

pub fn custom_accent(cx: &App) -> Hsla {
    cx.global::<AppThemeState>().colors.accent
}

pub fn font_color(cx: &App) -> Option<Hsla> {
    cx.global::<AppThemeState>().colors.font
}

pub fn background_color(cx: &App) -> Option<Hsla> {
    cx.global::<AppThemeState>().colors.background
}

pub fn hover_color(cx: &App) -> Option<Hsla> {
    cx.global::<AppThemeState>().colors.hover
}

pub fn selected_color(cx: &App) -> Option<Hsla> {
    cx.global::<AppThemeState>().colors.selected
}

pub fn wallpaper(cx: &App) -> Option<(PathBuf, f32)> {
    let visual = &cx.global::<AppThemeState>().visual;
    visual
        .wallpaper
        .clone()
        .map(|path| (path, visual.wallpaper_opacity))
}

pub fn wallpaper_opacity(cx: &App) -> f32 {
    cx.global::<AppThemeState>().visual.wallpaper_opacity
}

pub fn styles(cx: &App) -> CustomThemeStyles {
    let state = cx.global::<AppThemeState>();
    let colors = cx.theme();

    CustomThemeStyles {
        background: colors.background,
        foreground: colors.foreground,
        hover: colors.list_hover,
        selected: colors.selection,
        window_background: colors.background,
        window_background_img: state.visual.wallpaper.clone(),
    }
}

pub fn panel_background(cx: &App) -> Hsla {
    cx.theme()
        .background
        .opacity(if has_wallpaper(cx) { 0.15 } else { 1. })
}

pub fn border_color(cx: &App) -> Hsla {
    cx.theme()
        .border
        .opacity(if has_wallpaper(cx) { 0.62 } else { 0.9 })
}

pub fn tab_background(cx: &App) -> Hsla {
    if has_wallpaper(cx) {
        Hsla::transparent_black()
    } else {
        cx.theme().tab_bar
    }
}

pub fn title_background(cx: &App) -> Hsla {
    if has_wallpaper(cx) {
        cx.theme().title_bar.opacity(0.15)
    } else {
        cx.theme().title_bar
    }
}

pub fn sidebar_background(cx: &App) -> Hsla {
    if has_wallpaper(cx) {
        cx.theme().sidebar.opacity(0.15)
    } else {
        cx.theme().sidebar
    }
}

pub fn terminal_background(cx: &App) -> Hsla {
    if has_wallpaper(cx) {
        Hsla::transparent_black()
    } else {
        cx.theme().background
    }
}

pub fn terminal_foreground(cx: &App) -> Option<Hsla> {
    Some(cx.theme().foreground)
}

fn has_wallpaper(cx: &App) -> bool {
    cx.global::<AppThemeState>().visual.wallpaper.is_some()
}

pub fn window_background_appearance(_: &App) -> WindowBackgroundAppearance {
    WindowBackgroundAppearance::Transparent
}

pub fn set_font_color(color: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = AppTheme::Custom;
    state.colors.font = Some(color);
    refresh(cx, true);
}

pub fn clear_font_color(cx: &mut App) {
    cx.global_mut::<AppThemeState>().colors.font = None;
    refresh(cx, true);
}

pub fn set_background_color(color: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = AppTheme::Custom;
    state.colors.background = Some(color);
    refresh(cx, true);
}

pub fn clear_background_color(cx: &mut App) {
    cx.global_mut::<AppThemeState>().colors.background = None;
    refresh(cx, true);
}

pub fn set_hover_color(color: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = AppTheme::Custom;
    state.colors.hover = Some(color);
    refresh(cx, true);
}

pub fn clear_hover_color(cx: &mut App) {
    cx.global_mut::<AppThemeState>().colors.hover = None;
    refresh(cx, true);
}

pub fn set_selected_color(color: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = AppTheme::Custom;
    state.colors.selected = Some(color);
    refresh(cx, true);
}

pub fn clear_selected_color(cx: &mut App) {
    cx.global_mut::<AppThemeState>().colors.selected = None;
    refresh(cx, true);
}

pub fn set_wallpaper_opacity(opacity: f32, cx: &mut App) {
    cx.global_mut::<AppThemeState>().visual.wallpaper_opacity = opacity.clamp(0., 1.);
    refresh(cx, false);
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
    cx.global_mut::<AppThemeState>().visual.wallpaper = Some(target);
    refresh(cx, false);
    Ok(())
}

pub fn clear_wallpaper(cx: &mut App) {
    cx.global_mut::<AppThemeState>().visual.wallpaper = None;
    refresh(cx, false);
}

pub fn select(theme: AppTheme, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = theme;
    if theme != AppTheme::Custom {
        state.colors.font = None;
        state.colors.background = None;
        state.colors.hover = None;
        state.colors.selected = None;
    }
    refresh(cx, true);
}

pub fn select_custom(accent: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.theme = AppTheme::Custom;
    state.colors.accent = accent;
    refresh(cx, true);
}

fn apply_theme(cx: &mut App) {
    let (palette, colors) = {
        let state = cx.global::<AppThemeState>();
        let palette = if state.theme == AppTheme::Custom {
            custom_palette(state.colors.accent)
        } else {
            state.theme.palette()
        };
        let colors = if state.theme == AppTheme::Custom {
            state.colors
        } else {
            ColorOverrides {
                accent: state.colors.accent,
                ..Default::default()
            }
        };
        (palette, colors)
    };

    let accent: Hsla = palette.accent.into();
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
        .unwrap_or_else(|| derive_selected(background));

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
    theme.primary_hover = with_alpha(accent, 0.88);
    theme.primary_active = with_alpha(accent, 0.78);
    theme.primary_foreground = rgb_to_u32(255, 255, 255).into();
    theme.accent = soft;
    theme.accent_foreground = accent;
    theme.ring = with_alpha(accent, 0.45);
    theme.selection = selected;
    theme.link = accent;
    theme.link_hover = with_alpha(accent, 0.82);
    theme.link_active = with_alpha(accent, 0.72);
    theme.sidebar_primary = accent;
    theme.sidebar_primary_foreground = rgb_to_u32(255, 255, 255).into();
    theme.sidebar_accent = soft;
    theme.sidebar_accent_foreground = accent;
    theme.list_active = selected;
    theme.list_active_border = with_alpha(accent, 0.35);
    theme.list_hover = hover;
    theme.caret = accent;
    theme.progress_bar = accent;
    theme.slider_thumb = accent;
    let colors = theme.colors;
    theme.tokens = (&colors).into();
}

fn custom_palette(accent: Hsla) -> ThemePalette {
    let dark = accent.l < 0.28;
    let background = accent;
    let soft = shift_background(background, if dark { 0.11 } else { -0.08 }, 0.88);
    ThemePalette {
        accent: accent.into(),
        soft: soft.into(),
        background: background.into(),
        surface: derive_surface(background).into(),
        sidebar: derive_sidebar(background).into(),
        border: derive_border(background).into(),
    }
}

fn default_foreground(dark: bool) -> Hsla {
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

fn derive_selected(background: Hsla) -> Hsla {
    shift_background(
        background,
        if background.l < 0.5 { 0.18 } else { -0.14 },
        0.92,
    )
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

fn refresh(cx: &mut App, reapply_theme: bool) {
    if reapply_theme {
        apply_theme(cx);
    }
    persist(cx);
    cx.refresh_windows();
}

fn load_settings() -> ThemeSettings {
    fs::read(SETTINGS_PATH)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn persist(cx: &mut App) {
    let settings = {
        let state = cx.global::<AppThemeState>();
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
    read_global_state(cx).update(cx, |_, cx| cx.emit(GlobalEvent::ThemeChanged));
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

fn default_accent() -> String {
    "#F97316".to_owned()
}

fn default_custom_color() -> Hsla {
    Hsla::parse_hex(&default_accent()).expect("默认自定义主题颜色必须有效")
}
