use anyhow::Context as _;
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

use super::{ConnectionAdapter, ConnectionFuture, DirectAdapter, Socks5Adapter};

impl ConnectionAdapter for DirectAdapter {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a> {
        Box::pin(async move {
            let stream = TcpStream::connect(target)
                .await
                .with_context(|| format!("连接目标 {}:{} 失败", target.0, target.1))?;
            Ok(Box::new(stream) as Box<dyn super::ProxyStream>)
        })
    }
}

impl ConnectionAdapter for Socks5Adapter {
    fn connect<'a>(&'a self, target: (&'a str, u16)) -> ConnectionFuture<'a> {
        Box::pin(async move {
            let proxy = &self.settings;
            let proxy_address = (proxy.host.as_str(), proxy.port);
            let stream = if proxy.username.is_empty() {
                Socks5Stream::connect(proxy_address, target).await
            } else {
                Socks5Stream::connect_with_password(
                    proxy_address,
                    target,
                    &proxy.username,
                    &proxy.password,
                )
                .await
            }
            .with_context(|| format!("连接 SOCKS5 代理 {}:{} 失败", proxy.host, proxy.port))?;
            Ok(Box::new(stream) as Box<dyn super::ProxyStream>)
        })
    }
}

pub(super) async fn connect(
    target: (&str, u16),
    proxy: Option<&super::ProxySettings>,
) -> anyhow::Result<Box<dyn super::ProxyStream>> {
    match proxy {
        Some(proxy) => Socks5Adapter::new(proxy.clone()).connect(target).await,
        None => DirectAdapter.connect(target).await,
    }
}
