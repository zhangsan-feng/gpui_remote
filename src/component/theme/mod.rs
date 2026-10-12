use std::path::PathBuf;

pub use crate::domain::theme::AppTheme;
use crate::domain::theme::{StoredColors, ThemeSettings};
use gpui_kit::{App, Global, Hsla};

mod core;
mod external;
mod ui;

pub use external::{CustomerTheme, CustomerUiColor, CustomerUiTheme};
pub use ui::GuiColor;

const MIN_SELECTION_LIGHTNESS_CONTRAST: f32 = 0.12;
const SELECTION_LIGHTNESS_OFFSET: f32 = 0.14;
const REGION_BACKGROUND_OFFSET: f32 = 0.02;
const MIN_HOVER_SELECTION_CONTRAST: f32 = 0.08;
const HOVER_LIGHTNESS_OFFSET: f32 = 0.05;

struct DefaultLightPalette {
    canvas: u32,
    surface: u32,
    sidebar: u32,
    border: u32,
    foreground: u32,
    muted_foreground: u32,
    primary: u32,
    primary_hover: u32,
    primary_active: u32,
    hover: u32,
    selected: u32,
    mint: u32,
    mint_foreground: u32,
    violet: u32,
    violet_foreground: u32,
    orange: u32,
    orange_hover: u32,
    orange_active: u32,
    orange_foreground: u32,
}

const DEFAULT_LIGHT_PALETTE: DefaultLightPalette = DefaultLightPalette {
    canvas: 0xdfeeff,
    surface: 0xeaf3ff,
    sidebar: 0xf1f7ff,
    border: 0xafd1fa,
    foreground: 0x16365d,
    muted_foreground: 0x597293,
    primary: 0x2272d1,
    primary_hover: 0x195faf,
    primary_active: 0x125098,
    hover: 0xdcebfd,
    selected: 0xb6d8ff,
    mint: 0xbaefd6,
    mint_foreground: 0x00694c,
    violet: 0xc3baef,
    violet_foreground: 0x33208a,
    orange: 0xefceba,
    orange_hover: 0xebc2ab,
    orange_active: 0xe5b397,
    orange_foreground: 0x873100,
};

impl AppTheme {
    pub const BUILT_IN: [Self; 6] = [
        Self::DefaultTheme,
        Self::RoseBerry,
        Self::LightBlue,
        Self::LightOrange,
        Self::LightPurple,
        Self::LightPink,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::RoseBerry => "粉玫红",
            Self::DefaultTheme => "默认",
            Self::LightBlue => "浅蓝色",
            Self::LightOrange => "浅橙色",
            Self::LightPurple => "浅紫色",
            Self::LightPink => "浅粉色",
            Self::Custom => "自定义配色",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::RoseBerry => "保留的粉玫红主题。",
            Self::DefaultTheme => "浅蓝底色、白色内容区，搭配青绿、紫色和橙色强调。",
            Self::LightBlue => "使用图片色阶生成的浅蓝色主题。",
            Self::LightOrange => "使用图片色阶生成的浅橙色主题。",
            Self::LightPurple => "使用图片色阶生成的浅紫色主题。",
            Self::LightPink => "使用图片色阶生成的浅粉色主题。",
            Self::Custom => "自定义主色并自动派生界面颜色。",
        }
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

struct CustomerUiThemeState {
    theme: AppTheme,
    colors: ColorOverrides,
    visual: VisualSettings,
    persist_settings: tokio::sync::watch::Sender<ThemeSettings>,
    settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Global for CustomerUiThemeState {}

#[derive(Clone, Copy)]
pub struct ThemePreview {
    pub accent: Hsla,
    pub background: Hsla,
    pub hover: Hsla,
}

pub struct ChangeComponentThemeColor;

pub(crate) fn init(
    cx: &mut App,
    settings: ThemeSettings,
    persist_settings: tokio::sync::watch::Sender<ThemeSettings>,
    settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    core::initialize(cx, settings, persist_settings, settings_changed);
    CustomerUiTheme::apply(cx);
}
