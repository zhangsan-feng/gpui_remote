use std::path::PathBuf;

use crate::domain::theme::ThemeSettings;

fn settings_path() -> std::path::PathBuf {
    crate::infrastructure::paths::get().data.join("theme.json")
}

pub(crate) async fn load() -> Result<ThemeSettings, String> {
    match tokio::fs::read(settings_path()).await {
        Ok(bytes) => {
            let mut settings: ThemeSettings = serde_json::from_slice(&bytes)
                .map_err(|error| format!("解析主题设置失败: {error}"))?;
            if let Some(wallpaper) = settings.wallpaper.as_mut() {
                let path = PathBuf::from(&*wallpaper);
                if path.is_relative() {
                    *wallpaper = crate::infrastructure::paths::get()
                        .root
                        .join(path)
                        .to_string_lossy()
                        .into_owned();
                }
            }
            Ok(settings)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(ThemeSettings::default()),
        Err(error) => Err(format!("读取主题设置失败: {error}")),
    }
}

pub(crate) async fn save(settings: ThemeSettings) -> Result<(), String> {
    if let Some(parent) = settings_path().parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("创建主题设置目录失败: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(&settings)
        .map_err(|error| format!("序列化主题设置失败: {error}"))?;
    tokio::fs::write(settings_path(), bytes)
        .await
        .map_err(|error| format!("写入主题设置失败: {error}"))
}

pub(crate) async fn copy_wallpaper(source: PathBuf) -> Result<PathBuf, String> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "无法识别图片格式".to_owned())?;
    let directory = crate::infrastructure::paths::get().data.join("background");
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("创建壁纸目录失败: {error}"))?;
    let target = directory.join(format!("wallpaper.{extension}"));
    tokio::fs::copy(source, &target)
        .await
        .map_err(|error| format!("复制壁纸失败: {error}"))?;
    Ok(target)
}
