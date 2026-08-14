use std::path::PathBuf;

use gpui::{App, Global, Hsla};
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

impl AppTheme {
    pub const BUILT_IN: [Self; 10] = [
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Wisteria => "默认主题",
            Self::SeaSalt => "海盐",
            Self::Moss => "苔藓",
            Self::WarmSand => "暖沙",
            Self::MaterialRed => "材质红",
            Self::MaterialPink => "材质粉",
            Self::MaterialDeepOrange => "材质深橙",
            Self::MaterialOrange => "材质橙",
            Self::MaterialAmber => "材质琥珀",
            Self::MaterialBrown => "材质棕",
            Self::Custom => "自定义配色",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Wisteria => "白色应用主题，柔和的浅色工作区。",
            Self::SeaSalt => "清爽的海蓝色交互配色。",
            Self::Moss => "柔和的绿色交互配色。",
            Self::WarmSand => "温暖的沙黄色交互配色。",
            Self::MaterialRed => "高对比的材质红色。",
            Self::MaterialPink => "轻快的材质粉色。",
            Self::MaterialDeepOrange => "饱和的深橙色。",
            Self::MaterialOrange => "明亮的材质橙色。",
            Self::MaterialAmber => "温暖的琥珀色。",
            Self::MaterialBrown => "稳重的材质棕色。",
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
