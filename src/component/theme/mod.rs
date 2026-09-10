use std::path::PathBuf;

use gpui_kit::{App, Global, Hsla};
use serde::{Deserialize, Serialize};

mod core;
mod external;
mod ui;

pub use external::{CustomerTheme, CustomerUiColor, CustomerUiTheme};
pub use ui::GuiColor;

const SETTINGS_PATH: &str = "data/theme.json";
const WALLPAPER_DIRECTORY: &str = "data/background";
const MIN_SELECTION_LIGHTNESS_CONTRAST: f32 = 0.12;
const SELECTION_LIGHTNESS_OFFSET: f32 = 0.14;
const REGION_BACKGROUND_OFFSET: f32 = 0.02;
const MIN_HOVER_SELECTION_CONTRAST: f32 = 0.08;
const HOVER_LIGHTNESS_OFFSET: f32 = 0.05;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppTheme {
    #[serde(
        alias = "monochrome",
        alias = "wisteria",
        alias = "sea_salt",
        alias = "moss",
        alias = "warm_sand",
        alias = "material_red",
        alias = "material_pink",
        alias = "material_deep_orange",
        alias = "material_orange",
        alias = "material_amber",
        alias = "material_brown",
        alias = "peach_cream",
        alias = "sunset_coral",
        alias = "pomegranate_tea"
    )]
    RoseBerry,
    #[default]
    DefaultTheme,
    LightBlue,
    LightOrange,
    LightPurple,
    LightPink,
    Custom,
}

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
}

impl Global for CustomerUiThemeState {}

#[derive(Clone, Copy)]
pub struct ThemePreview {
    pub accent: Hsla,
    pub background: Hsla,
    pub hover: Hsla,
}

pub struct ChangeComponentThemeColor;

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
            accent: core::default_accent(),
            font: None,
            background: None,
            hover: None,
            selected: None,
        }
    }
}

pub fn init(cx: &mut App) {
    core::initialize(cx);
    CustomerUiTheme::apply(cx);
}
