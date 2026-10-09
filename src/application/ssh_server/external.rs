use crate::{
    application::ApplicationResult, data_context::DATA_CONTEXT, infrastructure::INFRASTRUCTURE,
};

use super::core::normalize_and_validate_ssh_server_settings;

impl super::SshServerApplication {
    pub(crate) async fn initialize(
        &self,
        sftp_server: std::sync::Arc<crate::application::sftp_server::SftpServerApplication>,
    ) -> ApplicationResult<()> {
        let mut status = INFRASTRUCTURE.subscribe_ssh_server_status();
        let data_context = &*DATA_CONTEXT;
        data_context.set_ssh_server_status(status.borrow().clone());
        tokio::spawn(async move {
            while status.changed().await.is_ok() {
                data_context.set_ssh_server_status(status.borrow_and_update().clone());
            }
        });

        let mut settings = INFRASTRUCTURE
            .load_ssh_server_settings()
            .await
            .map_err(|error| {
                DATA_CONTEXT.set_ssh_server_status(crate::domain::ssh_server::SshServerStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                error
            })?;
        if let Err(error) = normalize_and_validate_ssh_server_settings(&mut settings) {
            DATA_CONTEXT.set_ssh_server_status(crate::domain::ssh_server::SshServerStatus {
                error: Some(error.clone()),
                ..Default::default()
            });
            return Err(error);
        }
        if settings.enabled {
            INFRASTRUCTURE
                .start_ssh_server(settings, sftp_server)
                .await
                .map_err(|error| {
                    DATA_CONTEXT.set_ssh_server_status(
                        crate::domain::ssh_server::SshServerStatus {
                            error: Some(error.clone()),
                            ..Default::default()
                        },
                    );
                    error
                })?;
        }
        Ok(())
    }

    pub(crate) async fn current_settings(
        &self,
    ) -> ApplicationResult<crate::domain::ssh_server::SshServerSettings> {
        INFRASTRUCTURE.load_ssh_server_settings().await
    }

    pub(crate) async fn update_settings(
        &self,
        mut settings: crate::domain::ssh_server::SshServerSettings,
        sftp_server: std::sync::Arc<crate::application::sftp_server::SftpServerApplication>,
    ) -> ApplicationResult<crate::domain::ssh_server::SshServerSettings> {
        normalize_and_validate_ssh_server_settings(&mut settings)?;
        INFRASTRUCTURE
            .update_ssh_server(settings.clone(), sftp_server)
            .await?;
        Ok(settings)
    }

    pub(crate) async fn shutdown(&self) {
        if let Err(error) = INFRASTRUCTURE.shutdown_ssh_server().await {
            log::warn!("关闭 SSH 服务失败: {error}");
        }
    }
}
