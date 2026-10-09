use serde::{Deserialize, Serialize};

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

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ThemeSettings {
    pub(crate) theme: AppTheme,
    pub(crate) colors: StoredColors,
    pub(crate) wallpaper: Option<String>,
    pub(crate) wallpaper_opacity: f32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct StoredColors {
    pub(crate) accent: String,
    pub(crate) font: Option<String>,
    pub(crate) background: Option<String>,
    pub(crate) hover: Option<String>,
    pub(crate) selected: Option<String>,
}

impl Default for StoredColors {
    fn default() -> Self {
        Self {
            accent: "#FF3A83".to_owned(),
            font: None,
            background: None,
            hover: None,
            selected: None,
        }
    }
}
