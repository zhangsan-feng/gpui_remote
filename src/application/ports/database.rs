use std::{future::Future, pin::Pin};

use anyhow::Result;

pub(crate) type DatabaseConnectFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn DatabaseConnection>>> + Send + 'a>>;
pub(crate) type DatabaseCloseFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

pub(crate) trait DatabaseConnection: Send + Sync {
    fn close(&self) -> DatabaseCloseFuture<'_>;
}
