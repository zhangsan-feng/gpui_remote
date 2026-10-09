use crate::{
    domain::http_proxy::{HttpProxySettings, HttpProxyStatus},
    infrastructure::InfrastructureContext,
};

use super::{HttpProxyController, HttpProxyFuture};

impl InfrastructureContext {
    pub(super) fn http_proxy_controller(&self) -> &HttpProxyController {
        &self.inner.http_proxy
    }
}

impl InfrastructureContext {
    pub(crate) fn load_http_proxy_settings(&self) -> HttpProxyFuture<HttpProxySettings> {
        let context = self.clone();
        Box::pin(async move { context.http_proxy_controller().load_settings().await })
    }

    pub(crate) fn start_http_proxy(
        &self,
        settings: HttpProxySettings,
    ) -> HttpProxyFuture<HttpProxyStatus> {
        let context = self.clone();
        Box::pin(async move { context.http_proxy_controller().start(settings).await })
    }

    pub(crate) fn update_http_proxy(
        &self,
        settings: HttpProxySettings,
    ) -> HttpProxyFuture<HttpProxyStatus> {
        let context = self.clone();
        Box::pin(async move { context.http_proxy_controller().update(settings).await })
    }

    pub(crate) fn shutdown_http_proxy(&self) -> HttpProxyFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            context.http_proxy_controller().shutdown().await;
            Ok(())
        })
    }

    pub(crate) fn subscribe_http_proxy_status(
        &self,
    ) -> tokio::sync::watch::Receiver<HttpProxyStatus> {
        self.http_proxy_controller().subscribe_status()
    }
}
