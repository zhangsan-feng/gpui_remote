use std::sync::atomic::Ordering;

use crate::{
    application::{Application, ApplicationResult},
    data_context::DATA_CONTEXT,
    infrastructure::INFRASTRUCTURE,
};

impl Application {
    pub(crate) async fn initialize(&self) -> ApplicationResult<()> {
        INFRASTRUCTURE.initialize().await?;
        if let Err(error) = self.port_forward.initialize().await {
            log::error!("初始化端口转发失败: {error}");
        }
        if let Err(error) = INFRASTRUCTURE.start_mcp(self.clone()).await {
            log::error!("启动基础设施 MCP 失败: {error}");
        }
        if let Err(error) = self.ssh_server.initialize(self.sftp_server.clone()).await {
            log::error!("启动 SSH 服务失败: {error}");
        }
        if let Err(error) = self.socks5_proxy.initialize().await {
            log::error!("启动 SOCKS5 服务失败: {error}");
        }
        if let Err(error) = self.http_proxy.initialize().await {
            log::error!("启动 HTTP 代理失败: {error}");
        }
        self.theme.initialize_runtime().await;
        Ok(())
    }

    /// Keep accepted GUI/MCP work alive until the explicit exit barrier completes.
    pub(crate) async fn begin_operation(
        &self,
    ) -> ApplicationResult<tokio::sync::OwnedRwLockReadGuard<()>> {
        let operation = self.operations.clone().read_owned().await;
        if self.shutting_down.load(Ordering::Acquire) {
            return Err("应用正在退出，操作未执行".to_owned());
        }
        Ok(operation)
    }

    pub(crate) async fn begin_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        let _operations = self.operations.write().await;
        log::info!("application_operations_drained: new_operations=rejected");
    }

    pub(crate) async fn shutdown(&self) {
        self.begin_shutdown().await;
        self.port_forward.shutdown().await;
        if let Err(error) = INFRASTRUCTURE.shutdown_mcp().await {
            log::warn!("关闭 MCP 服务失败: {error}");
        }
        self.theme.shutdown_runtime().await;

        for workspace in DATA_CONTEXT.workspace_snapshot().workspaces {
            if let Err(error) = self.close_session(&workspace.workspace_id).await {
                log::warn!(
                    "退出时清理工作区失败: workspace_id={}, error={error}",
                    workspace.workspace_id
                );
            }
        }
        self.ssh_server.shutdown().await;
        self.socks5_proxy.shutdown().await;
        self.http_proxy.shutdown().await;
    }
}
