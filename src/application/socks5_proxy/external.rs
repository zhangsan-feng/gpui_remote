use crate::{
    application::ApplicationResult, data_context::DATA_CONTEXT, infrastructure::INFRASTRUCTURE,
};

use super::core::normalize_and_validate_socks5_settings;

impl super::Socks5ProxyApplication {
    pub(crate) async fn initialize(&self) -> ApplicationResult<()> {
        let mut status = INFRASTRUCTURE.subscribe_socks5_proxy_status();
        let data_context = &*DATA_CONTEXT;
        data_context.set_socks5_proxy_status(status.borrow().clone());
        tokio::spawn(async move {
            while status.changed().await.is_ok() {
                data_context.set_socks5_proxy_status(status.borrow_and_update().clone());
            }
        });

        let mut settings = INFRASTRUCTURE
            .load_socks5_proxy_settings()
            .await
            .map_err(|error| {
                DATA_CONTEXT.set_socks5_proxy_status(
                    crate::domain::socks5_proxy::Socks5ProxyStatus {
                        error: Some(error.clone()),
                        ..Default::default()
                    },
                );
                error
            })?;
        if settings.enabled {
            if let Err(error) = normalize_and_validate_socks5_settings(&mut settings) {
                DATA_CONTEXT.set_socks5_proxy_status(
                    crate::domain::socks5_proxy::Socks5ProxyStatus {
                        error: Some(error.clone()),
                        ..Default::default()
                    },
                );
                return Err(error);
            }
            INFRASTRUCTURE
                .start_socks5_proxy(settings)
                .await
                .map_err(|error| {
                    DATA_CONTEXT.set_socks5_proxy_status(
                        crate::domain::socks5_proxy::Socks5ProxyStatus {
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
    ) -> ApplicationResult<crate::domain::socks5_proxy::Socks5ProxySettings> {
        INFRASTRUCTURE.load_socks5_proxy_settings().await
    }

    pub(crate) async fn update_settings(
        &self,
        mut settings: crate::domain::socks5_proxy::Socks5ProxySettings,
    ) -> ApplicationResult<crate::domain::socks5_proxy::Socks5ProxySettings> {
        normalize_and_validate_socks5_settings(&mut settings)?;
        INFRASTRUCTURE.update_socks5_proxy(settings.clone()).await?;
        Ok(settings)
    }

    pub(crate) async fn shutdown(&self) {
        if let Err(error) = INFRASTRUCTURE.shutdown_socks5_proxy().await {
            log::warn!("关闭 SOCKS5 服务失败: {error}");
        }
    }
}
