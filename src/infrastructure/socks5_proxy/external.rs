use crate::{
    application::ports::Socks5ProxyFuture,
    domain::socks5_proxy::{Socks5ProxySettings, Socks5ProxyStatus},
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) fn load_socks5_proxy_settings(&self) -> Socks5ProxyFuture<Socks5ProxySettings> {
        let context = self.clone();
        Box::pin(async move { context.inner.socks5_proxy.load_settings().await })
    }

    pub(crate) fn start_socks5_proxy(
        &self,
        settings: Socks5ProxySettings,
    ) -> Socks5ProxyFuture<Socks5ProxyStatus> {
        let context = self.clone();
        Box::pin(async move { context.inner.socks5_proxy.start(settings).await })
    }

    pub(crate) fn update_socks5_proxy(
        &self,
        settings: Socks5ProxySettings,
    ) -> Socks5ProxyFuture<Socks5ProxyStatus> {
        let context = self.clone();
        Box::pin(async move { context.inner.socks5_proxy.update(settings).await })
    }

    pub(crate) fn shutdown_socks5_proxy(&self) -> Socks5ProxyFuture<()> {
        let context = self.clone();
        Box::pin(async move {
            context.inner.socks5_proxy.shutdown().await;
            Ok(())
        })
    }

    pub(crate) fn subscribe_socks5_proxy_status(
        &self,
    ) -> tokio::sync::watch::Receiver<Socks5ProxyStatus> {
        self.inner.socks5_proxy.subscribe_status()
    }
}
