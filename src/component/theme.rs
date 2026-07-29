use std::{
    fs,
    path::{Path, PathBuf},
};

use gpui::{App, Global, Hsla, Rgba};
use gpui_component::{Colorize, Theme, ThemeMode};
use serde::{Deserialize, Serialize};

use crate::component::color::rgb_to_u32;
use crate::global_state::{GlobalEvent, read_global_state};

const SETTINGS_PATH: &str = "data/settings.json";

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
pub struct ThemePreset {
    pub id: AppTheme,
    pub name: &'static str,
    pub description: &'static str,
    pub accent: Rgba,
    pub soft: Rgba,
    pub background: Rgba,
    pub surface: Rgba,
    pub sidebar: Rgba,
    pub border: Rgba,
}

impl AppTheme {
    pub const ALL: [Self; 10] = [
        Self::Wisteria,
        Self::SeaSalt,
        Self::Moss,
        Self::WarmSand,
        Self::MaterialRed,
        Self::MaterialPink,
        Self::MaterialDeepOrange,
        Self::MaterialOrange,
        Self::MaterialAmber,
        Self::MaterialBrown,
    ];

    pub fn preset(self) -> ThemePreset {
        match self {
            Self::Wisteria => ThemePreset {
                id: self,
                name: "紫藤",
                description: "柔和沉稳的紫灰色",
                accent: rgb_to_u32(124, 58, 237),
                soft: rgb_to_u32(237, 233, 254),
                background: rgb_to_u32(253, 252, 254),
                surface: rgb_to_u32(250, 247, 252),
                sidebar: rgb_to_u32(247, 243, 249),
                border: rgb_to_u32(230, 224, 235),
            },
            Self::SeaSalt => ThemePreset {
                id: self,
                name: "海盐蓝",
                description: "清爽专业的海蓝色",
                accent: rgb_to_u32(2, 132, 199),
                soft: rgb_to_u32(224, 242, 254),
                background: rgb_to_u32(248, 252, 254),
                surface: rgb_to_u32(240, 249, 255),
                sidebar: rgb_to_u32(241, 248, 252),
                border: rgb_to_u32(210, 229, 238),
            },
            Self::Moss => ThemePreset {
                id: self,
                name: "青苔绿",
                description: "舒缓自然的青绿色",
                accent: rgb_to_u32(5, 150, 105),
                soft: rgb_to_u32(209, 250, 229),
                background: rgb_to_u32(249, 253, 251),
                surface: rgb_to_u32(240, 253, 247),
                sidebar: rgb_to_u32(241, 249, 245),
                border: rgb_to_u32(211, 230, 220),
            },
            Self::WarmSand => ThemePreset {
                id: self,
                name: "暖砂橙",
                description: "温暖克制的琥珀色",
                accent: rgb_to_u32(217, 119, 6),
                soft: rgb_to_u32(254, 243, 199),
                background: rgb_to_u32(255, 253, 248),
                surface: rgb_to_u32(255, 248, 235),
                sidebar: rgb_to_u32(250, 246, 237),
                border: rgb_to_u32(235, 222, 197),
            },
            Self::MaterialRed => material_preset(
                self,
                "Material Red",
                "鲜明有力的暖红色",
                (244, 67, 54),
                (255, 235, 238),
            ),
            Self::MaterialPink => material_preset(
                self,
                "Material Pink",
                "柔和活跃的玫粉色",
                (233, 30, 99),
                (252, 228, 236),
            ),
            Self::MaterialDeepOrange => material_preset(
                self,
                "Material Deep Orange",
                "浓郁醒目的深橙色",
                (255, 87, 34),
                (251, 233, 231),
            ),
            Self::MaterialOrange => material_preset(
                self,
                "Material Orange",
                "明快温暖的橙色",
                (255, 152, 0),
                (255, 243, 224),
            ),
            Self::MaterialAmber => material_preset(
                self,
                "Material Amber",
                "明亮柔和的琥珀色",
                (255, 193, 7),
                (255, 248, 225),
            ),
            Self::MaterialBrown => material_preset(
                self,
                "Material Brown",
                "稳定自然的棕色",
                (121, 85, 72),
                (239, 235, 233),
            ),
            Self::Custom => unreachable!("自定义主题由取色器生成"),
        }
    }
}

fn material_preset(
    id: AppTheme,
    name: &'static str,
    description: &'static str,
    accent: (u8, u8, u8),
    soft: (u8, u8, u8),
) -> ThemePreset {
    ThemePreset {
        id,
        name,
        description,
        accent: rgb_to_u32(accent.0, accent.1, accent.2),
        soft: rgb_to_u32(soft.0, soft.1, soft.2),
        background: rgb_to_u32(255, 253, 251),
        surface: rgb_to_u32(253, 248, 245),
        sidebar: rgb_to_u32(249, 244, 241),
        border: rgb_to_u32(232, 220, 214),
    }
}

#[derive(Serialize, Deserialize)]
struct AppSettings {
    theme: AppTheme,
    #[serde(default = "default_custom_accent")]
    custom_accent: String,
    #[serde(default = "default_sidebar_background")]
    sidebar_background: String,
    #[serde(default = "default_terminal_background")]
    terminal_background: String,
    #[serde(default = "default_panel_opacity")]
    sidebar_opacity: f32,
    #[serde(default = "default_panel_opacity")]
    terminal_opacity: f32,
    #[serde(default)]
    wallpaper_path: Option<String>,
    #[serde(default = "default_wallpaper_opacity")]
    wallpaper_opacity: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: AppTheme::default(),
            custom_accent: default_custom_accent(),
            sidebar_background: default_sidebar_background(),
            terminal_background: default_terminal_background(),
            sidebar_opacity: default_panel_opacity(),
            terminal_opacity: default_panel_opacity(),
            wallpaper_path: None,
            wallpaper_opacity: default_wallpaper_opacity(),
        }
    }
}

struct AppThemeState {
    selected: AppTheme,
    custom_accent: Hsla,
    sidebar_background: Hsla,
    terminal_background: Hsla,
    sidebar_opacity: f32,
    terminal_opacity: f32,
    wallpaper_path: Option<PathBuf>,
    wallpaper_opacity: f32,
}

impl Global for AppThemeState {}

pub fn init(cx: &mut App) {
    let settings = load_settings();
    let custom_accent =
        Hsla::parse_hex(&settings.custom_accent).unwrap_or_else(|_| default_custom_color());
    let sidebar_background = parse_color_or(&settings.sidebar_background, "#171311");
    let terminal_background = parse_color_or(&settings.terminal_background, "#141218");
    cx.set_global(AppThemeState {
        selected: settings.theme,
        custom_accent,
        sidebar_background,
        terminal_background,
        sidebar_opacity: settings.sidebar_opacity.clamp(0., 1.),
        terminal_opacity: settings.terminal_opacity.clamp(0., 1.),
        wallpaper_path: settings.wallpaper_path.map(PathBuf::from),
        wallpaper_opacity: settings.wallpaper_opacity.clamp(0., 1.),
    });
    apply_active(cx);
}

pub fn active(cx: &App) -> AppTheme {
    cx.global::<AppThemeState>().selected
}

pub fn custom_accent(cx: &App) -> Hsla {
    cx.global::<AppThemeState>().custom_accent
}

pub fn sidebar_color(cx: &App) -> Hsla {
    let state = cx.global::<AppThemeState>();
    if state.wallpaper_path.is_some() {
        return Hsla::transparent_black();
    }
    with_alpha(state.sidebar_background, state.sidebar_opacity)
}

pub fn sidebar_base_color(cx: &App) -> Hsla {
    cx.global::<AppThemeState>().sidebar_background
}

pub fn sidebar_opacity(cx: &App) -> f32 {
    cx.global::<AppThemeState>().sidebar_opacity
}

pub fn terminal_color(cx: &App) -> Hsla {
    let state = cx.global::<AppThemeState>();
    if state.wallpaper_path.is_some() {
        return Hsla::transparent_black();
    }
    with_alpha(state.terminal_background, state.terminal_opacity)
}

pub fn terminal_base_color(cx: &App) -> Hsla {
    cx.global::<AppThemeState>().terminal_background
}

pub fn terminal_opacity(cx: &App) -> f32 {
    cx.global::<AppThemeState>().terminal_opacity
}

pub fn wallpaper(cx: &App) -> Option<(PathBuf, f32)> {
    let state = cx.global::<AppThemeState>();
    state
        .wallpaper_path
        .clone()
        .map(|path| (path, state.wallpaper_opacity))
}

pub fn wallpaper_opacity(cx: &App) -> f32 {
    cx.global::<AppThemeState>().wallpaper_opacity
}

pub fn set_sidebar_color(color: Hsla, cx: &mut App) {
    cx.global_mut::<AppThemeState>().sidebar_background = color;
    persist_and_refresh(cx);
}

pub fn set_terminal_color(color: Hsla, cx: &mut App) {
    cx.global_mut::<AppThemeState>().terminal_background = color;
    persist_and_refresh(cx);
}

pub fn set_sidebar_opacity(opacity: f32, cx: &mut App) {
    cx.global_mut::<AppThemeState>().sidebar_opacity = opacity.clamp(0., 1.);
    persist_and_refresh(cx);
}

pub fn set_terminal_opacity(opacity: f32, cx: &mut App) {
    cx.global_mut::<AppThemeState>().terminal_opacity = opacity.clamp(0., 1.);
    persist_and_refresh(cx);
}

pub fn set_wallpaper_opacity(opacity: f32, cx: &mut App) {
    cx.global_mut::<AppThemeState>().wallpaper_opacity = opacity.clamp(0., 1.);
    persist_and_refresh(cx);
}

pub fn set_wallpaper(source: &Path, cx: &mut App) -> Result<(), String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "无法识别图片格式".to_owned())?;
    if !["png", "jpg", "jpeg", "webp"].contains(&extension.as_str()) {
        return Err("仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned());
    }

    let directory = Path::new("data/background");
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let target = directory.join(format!("wallpaper.{extension}"));
    fs::copy(source, &target).map_err(|error| error.to_string())?;
    cx.global_mut::<AppThemeState>().wallpaper_path = Some(target);
    persist_and_refresh(cx);
    Ok(())
}

pub fn clear_wallpaper(cx: &mut App) {
    cx.global_mut::<AppThemeState>().wallpaper_path = None;
    persist_and_refresh(cx);
}

pub fn select(selected: AppTheme, cx: &mut App) {
    cx.global_mut::<AppThemeState>().selected = selected;
    apply_active(cx);
    persist_and_refresh(cx);
}

pub fn select_custom(accent: Hsla, cx: &mut App) {
    let state = cx.global_mut::<AppThemeState>();
    state.selected = AppTheme::Custom;
    state.custom_accent = accent;
    apply_active(cx);
    persist_and_refresh(cx);
}

fn apply_active(cx: &mut App) {
    let state = cx.global::<AppThemeState>();
    let preset = if state.selected == AppTheme::Custom {
        custom_preset(state.custom_accent)
    } else {
        state.selected.preset()
    };
    apply(preset, cx);
}

fn custom_preset(accent: Hsla) -> ThemePreset {
    let tone = |saturation: f32, lightness: f32| Hsla {
        h: accent.h,
        s: saturation,
        l: lightness,
        a: 1.,
    };
    let dark = accent.l < 0.28;
    ThemePreset {
        id: AppTheme::Custom,
        name: "自定义",
        description: "由主色自动生成",
        accent: accent.into(),
        soft: tone(
            accent.s.min(if dark { 0.38 } else { 0.5 }),
            if dark { 0.17 } else { 0.94 },
        )
        .into(),
        background: tone(
            accent.s.min(if dark { 0.22 } else { 0.2 }),
            if dark { 0.065 } else { 0.985 },
        )
        .into(),
        surface: tone(
            accent.s.min(if dark { 0.28 } else { 0.28 }),
            if dark { 0.095 } else { 0.965 },
        )
        .into(),
        sidebar: tone(
            accent.s.min(if dark { 0.24 } else { 0.24 }),
            if dark { 0.115 } else { 0.95 },
        )
        .into(),
        border: tone(
            accent.s.min(if dark { 0.3 } else { 0.3 }),
            if dark { 0.22 } else { 0.86 },
        )
        .into(),
    }
}

fn apply(preset: ThemePreset, cx: &mut App) {
    let theme = Theme::global_mut(cx);
    let accent = preset.accent.into();
    let soft = preset.soft.into();
    let background: Hsla = preset.background.into();
    let surface = preset.surface.into();
    let sidebar = preset.sidebar.into();
    let border = preset.border.into();
    let dark = background.l < 0.5;
    let foreground = if dark {
        rgb_to_u32(245, 243, 242).into()
    } else {
        rgb_to_u32(48, 43, 55).into()
    };
    let muted_foreground = if dark {
        rgb_to_u32(184, 176, 172).into()
    } else {
        rgb_to_u32(107, 99, 120).into()
    };

    theme.mode = if dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };
    theme.background = background;
    theme.foreground = foreground;
    theme.muted_foreground = muted_foreground;
    theme.muted = surface;
    theme.secondary = surface;
    theme.secondary_foreground = foreground;
    theme.secondary_hover = soft;
    theme.secondary_active = soft;
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
    theme.selection = with_alpha(accent, 0.2);
    theme.link = accent;
    theme.link_hover = with_alpha(accent, 0.82);
    theme.link_active = with_alpha(accent, 0.72);
    theme.sidebar_primary = accent;
    theme.sidebar_primary_foreground = rgb_to_u32(255, 255, 255).into();
    theme.sidebar_accent = soft;
    theme.sidebar_accent_foreground = accent;
    theme.list_active = soft;
    theme.list_active_border = with_alpha(accent, 0.35);
    theme.list_hover = soft;
    theme.caret = accent;
    theme.progress_bar = accent;
    theme.slider_thumb = accent;
}

fn with_alpha(mut color: gpui::Hsla, alpha: f32) -> gpui::Hsla {
    color.a = alpha;
    color
}

fn load_settings() -> AppSettings {
    fs::read(SETTINGS_PATH)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn persist_and_refresh(cx: &mut App) {
    let state = cx.global::<AppThemeState>();
    let settings = AppSettings {
        theme: state.selected,
        custom_accent: state.custom_accent.to_hex(),
        sidebar_background: state.sidebar_background.to_hex(),
        terminal_background: state.terminal_background.to_hex(),
        sidebar_opacity: state.sidebar_opacity,
        terminal_opacity: state.terminal_opacity,
        wallpaper_path: state
            .wallpaper_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
        wallpaper_opacity: state.wallpaper_opacity,
    };
    if let Err(error) = save_settings(&settings) {
        log::error!("保存主题设置失败: {error}");
    }
    read_global_state(cx).update(cx, |_, cx| cx.emit(GlobalEvent::ThemeChanged));
    cx.refresh_windows();
}

fn save_settings(settings: &AppSettings) -> std::io::Result<()> {
    if let Some(parent) = Path::new(SETTINGS_PATH).parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(SETTINGS_PATH, bytes)
}

fn default_custom_accent() -> String {
    "#F97316".to_owned()
}

fn default_custom_color() -> Hsla {
    Hsla::parse_hex(&default_custom_accent()).expect("默认自定义主题颜色必须有效")
}

fn default_sidebar_background() -> String {
    "#171311".to_owned()
}

fn default_terminal_background() -> String {
    "#141218".to_owned()
}

fn default_panel_opacity() -> f32 {
    0.88
}

fn default_wallpaper_opacity() -> f32 {
    0.72
}

fn parse_color_or(value: &str, fallback: &str) -> Hsla {
    Hsla::parse_hex(value)
        .or_else(|_| Hsla::parse_hex(fallback))
        .expect("内置区域背景颜色必须有效")
}
