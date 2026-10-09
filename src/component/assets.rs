use std::borrow::Cow;

use gpui_kit::*;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "./src/icon"]
struct AssetFiles;

pub(crate) struct MergedAssets {
    component_assets: gpui_kit::assets::Assets,
}

impl MergedAssets {
    pub(crate) fn new() -> Self {
        Self {
            component_assets: gpui_kit::assets::Assets,
        }
    }
}

impl AssetSource for MergedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let clean_path = path.trim_start_matches("icon/").trim_start_matches('/');
        if let Some(file) = AssetFiles::get(clean_path) {
            return Ok(Some(file.data));
        }

        self.component_assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all_files = std::collections::HashSet::new();
        let clean_path = path.trim_start_matches("icon/").trim_start_matches('/');
        for file_path in AssetFiles::iter() {
            if file_path.starts_with(clean_path) {
                all_files.insert(file_path.to_string());
            }
        }

        for file_path in self.component_assets.list(path)? {
            all_files.insert(file_path.to_string());
        }

        Ok(all_files.into_iter().map(SharedString::from).collect())
    }
}
