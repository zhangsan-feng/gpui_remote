use crate::infrastructure::{InfrastructureContext, Storage};

impl InfrastructureContext {
    pub(crate) async fn initialize(&self) -> Result<(), String> {
        self.inner
            .storage
            .get_or_try_init(|| async {
                tokio::task::spawn_blocking(Storage::new)
                    .await
                    .map_err(|error| format!("初始化 SQLite 任务失败: {error}"))?
                    .map_err(|error| format!("初始化 SQLite 失败: {error:#}"))
            })
            .await?;
        log::info!("infrastructure_initialized");
        Ok(())
    }
}
