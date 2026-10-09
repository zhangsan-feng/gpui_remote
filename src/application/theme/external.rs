use crate::{application::ApplicationResult, infrastructure::INFRASTRUCTURE};

use super::{ThemeApplication, core::validate_wallpaper};

impl ThemeApplication {
    pub(crate) async fn initialize_runtime(&self) {
        if self.runtime.lock().await.is_some() {
            return;
        }

        let settings = self.load_settings().await;
        let (updates, receiver) = tokio::sync::watch::channel(settings.clone());
        let settings_changed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (shutdown, shutdown_receiver) = tokio::sync::oneshot::channel();
        let application = self.clone();
        let worker_settings_changed = settings_changed.clone();
        let worker = tokio::spawn(async move {
            application
                .run_settings_persistence(receiver, shutdown_receiver, worker_settings_changed)
                .await;
        });

        *self.runtime.lock().await = Some(super::ThemeSettingsRuntime {
            settings,
            updates,
            settings_changed,
            shutdown: Some(shutdown),
            worker,
        });
    }

    pub(crate) async fn settings(&self) -> crate::domain::theme::ThemeSettings {
        self.runtime
            .lock()
            .await
            .as_ref()
            .expect("theme runtime is not initialized")
            .settings
            .clone()
    }

    pub(crate) async fn settings_updates(
        &self,
    ) -> tokio::sync::watch::Sender<crate::domain::theme::ThemeSettings> {
        self.runtime
            .lock()
            .await
            .as_ref()
            .expect("theme runtime is not initialized")
            .updates
            .clone()
    }

    pub(crate) async fn settings_changed(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.runtime
            .lock()
            .await
            .as_ref()
            .expect("theme runtime is not initialized")
            .settings_changed
            .clone()
    }

    pub(crate) async fn shutdown_runtime(&self) {
        let Some(mut runtime) = self.runtime.lock().await.take() else {
            return;
        };
        if let Some(shutdown) = runtime.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Err(error) = runtime.worker.await {
            log::error!("等待主题设置保存任务失败: {error}");
        }
    }

    pub(crate) async fn load_settings(&self) -> crate::domain::theme::ThemeSettings {
        INFRASTRUCTURE
            .load_theme_settings()
            .await
            .unwrap_or_else(|error| {
                log::warn!("读取主题设置失败，使用默认设置: {error}");
                crate::domain::theme::ThemeSettings::default()
            })
    }

    pub(crate) async fn save_settings(
        &self,
        settings: crate::domain::theme::ThemeSettings,
    ) -> ApplicationResult<()> {
        INFRASTRUCTURE.save_theme_settings(settings).await
    }

    pub(crate) async fn run_settings_persistence(
        &self,
        mut settings: tokio::sync::watch::Receiver<crate::domain::theme::ThemeSettings>,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
        settings_changed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        loop {
            tokio::select! {
                changed = settings.changed() => {
                    if changed.is_err() {
                        break;
                    }
                    let current = settings.borrow_and_update().clone();
                    if let Err(error) = self.save_settings(current).await {
                        log::error!("保存主题设置失败: {error}");
                    }
                }
                _ = &mut shutdown => {
                    break;
                }
            }
        }
        if settings_changed.load(std::sync::atomic::Ordering::Acquire) {
            let current = settings.borrow_and_update().clone();
            if let Err(error) = self.save_settings(current).await {
                log::error!("退出前保存主题设置失败: {error}");
            }
        }
    }

    pub(crate) async fn copy_wallpaper(
        &self,
        source: std::path::PathBuf,
    ) -> ApplicationResult<std::path::PathBuf> {
        validate_wallpaper(&source)?;
        INFRASTRUCTURE.copy_theme_wallpaper(source).await
    }
}
