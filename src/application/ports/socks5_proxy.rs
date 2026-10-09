use std::{future::Future, pin::Pin};

pub(crate) type Socks5ProxyFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;
