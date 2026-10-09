use std::{future::Future, pin::Pin};

pub(crate) type SessionRepositoryFuture<T> =
    Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>;
