use anyhow::{Context as _, Result};
use tokio::net::TcpStream;
use tokio_socks::tcp::Socks5Stream;

pub(super) async fn connect(
    target: (&str, u16),
    proxy: Option<&super::ProxySettings>,
) -> Result<Box<dyn super::ProxyStream>> {
    if let Some(proxy) = proxy {
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
        return Ok(Box::new(stream));
    }

    let stream = TcpStream::connect(target)
        .await
        .with_context(|| format!("连接目标 {}:{} 失败", target.0, target.1))?;
    Ok(Box::new(stream))
}
