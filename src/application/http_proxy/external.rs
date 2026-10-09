use crate::{
    data_context::DATA_CONTEXT, domain::http_proxy::HttpProxySettings,
    infrastructure::INFRASTRUCTURE,
};

use super::{
    HttpProxyApplication,
    core::{normalize_and_validate_settings, publish_error},
};
use crate::application::ApplicationResult;

impl HttpProxyApplication {
    pub(crate) async fn initialize(&self) -> ApplicationResult<()> {
        let mut status = INFRASTRUCTURE.subscribe_http_proxy_status();
        let data_context = &*DATA_CONTEXT;
        data_context.set_http_proxy_status(status.borrow_and_update().clone());
        tokio::spawn(async move {
            while status.changed().await.is_ok() {
                data_context.set_http_proxy_status(status.borrow_and_update().clone());
            }
        });

        let mut settings = INFRASTRUCTURE
            .load_http_proxy_settings()
            .await
            .map_err(publish_error)?;
        normalize_and_validate_settings(&mut settings).map_err(publish_error)?;
        if settings.enabled {
            INFRASTRUCTURE.start_http_proxy(settings).await?;
        }
        Ok(())
    }

    pub(crate) async fn shutdown(&self) {
        if let Err(error) = INFRASTRUCTURE.shutdown_http_proxy().await {
            log::warn!("关闭 HTTP 代理失败: {error}");
        }
    }

    pub(crate) async fn current_settings(&self) -> ApplicationResult<HttpProxySettings> {
        INFRASTRUCTURE.load_http_proxy_settings().await
    }

    pub(crate) async fn update_settings(
        &self,
        mut settings: HttpProxySettings,
    ) -> ApplicationResult<HttpProxySettings> {
        normalize_and_validate_settings(&mut settings)?;
        INFRASTRUCTURE.update_http_proxy(settings.clone()).await?;
        Ok(settings)
    }
}
