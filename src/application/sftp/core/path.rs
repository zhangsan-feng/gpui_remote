use std::path::PathBuf;

pub(crate) fn default_desktop_path() -> PathBuf {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let desktop = profile.join("Desktop");
    if desktop.is_dir() { desktop } else { profile }
}
