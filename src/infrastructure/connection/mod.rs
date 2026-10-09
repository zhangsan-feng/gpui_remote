mod external;

use std::{future::Future, pin::Pin};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::domain::session::ProxyConfig;

pub trait ConnectionStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> ConnectionStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

pub(super) type ConnectionFuture<'a> =
    Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn ConnectionStream>>> + Send + 'a>>;

pub(super) trait ConnectionPort: Send + Sync {
    fn connect<'a>(
        &'a self,
        host: &'a str,
        port: u16,
        proxy: Option<&'a ProxyConfig>,
    ) -> ConnectionFuture<'a>;
}
