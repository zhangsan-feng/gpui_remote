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
            Self::DefaultTheme => "黑白、浅灰、深灰的默认主题。",
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
