use std::path::PathBuf;

pub use crate::domain::theme::AppTheme;
use crate::domain::theme::{StoredColors, ThemeSettings};
use gpui_kit::{App, Global, Hsla};

mod core;
mod external;

pub use external::{CustomerTheme, CustomerUiColor};

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
            Self::DefaultTheme => "使用 GPUI Kit 内置主题。",
            Self::LightBlue => "使用浅蓝色背景和主色。",
            Self::LightOrange => "使用浅橙色背景和主色。",
            Self::LightPurple => "使用浅紫色背景和主色。",
            Self::LightPink => "使用浅粉色背景和主色。",
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

pub(crate) fn init(
    cx: &mut App,
    settings: ThemeSettings,
    persist_settings: tokio::sync::watch::Sender<ThemeSettings>,
    settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    core::initialize(cx, settings, persist_settings, settings_changed);
    core::apply_theme(cx);
}
