use crate::{domain::session::ProxyConfig, infrastructure::InfrastructureContext};

use super::{ConnectionFuture, ConnectionPort, ConnectionStream};

impl ConnectionPort for InfrastructureContext {
    fn connect<'a>(
        &'a self,
        host: &'a str,
        port: u16,
        proxy: Option<&'a ProxyConfig>,
    ) -> ConnectionFuture<'a> {
        Box::pin(async move {
            let proxy_settings = proxy.map(|proxy| crate::infrastructure::proxy::ProxySettings {
                host: proxy.host.clone(),
                port: proxy.port,
                username: proxy.username.clone(),
                password: proxy.password.clone(),
            });
            let stream =
                crate::infrastructure::proxy::connect((host, port), proxy_settings.as_ref())
                    .await?;
            let stream: Box<dyn ConnectionStream> = stream;
            Ok(stream)
        })
    }
}
