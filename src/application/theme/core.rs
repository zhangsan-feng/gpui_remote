use crate::application::ApplicationResult;

pub(super) fn validate_wallpaper(source: &std::path::Path) -> ApplicationResult<()> {
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "无法识别图片格式".to_owned())?;
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err("仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned());
    }
    Ok(())
}
