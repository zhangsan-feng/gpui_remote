mod core;
mod external;

#[derive(Clone, Default)]
pub(crate) struct ThemeApplication {
    runtime: std::sync::Arc<tokio::sync::Mutex<Option<ThemeSettingsRuntime>>>,
}

struct ThemeSettingsRuntime {
    settings: crate::domain::theme::ThemeSettings,
    updates: tokio::sync::watch::Sender<crate::domain::theme::ThemeSettings>,
    settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    worker: tokio::task::JoinHandle<()>,
}

impl ThemeApplication {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}
