use std::path::PathBuf;

use crate::{
    application::ports::ThemeSettingsFuture, domain::theme::ThemeSettings,
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn load_theme_settings(&self) -> ThemeSettingsFuture<ThemeSettings> {
        Box::pin(super::load())
    }

    pub(crate) fn save_theme_settings(&self, settings: ThemeSettings) -> ThemeSettingsFuture<()> {
        Box::pin(super::save(settings))
    }

    pub(crate) fn copy_theme_wallpaper(&self, source: PathBuf) -> ThemeSettingsFuture<PathBuf> {
        Box::pin(super::copy_wallpaper(source))
    }
}
