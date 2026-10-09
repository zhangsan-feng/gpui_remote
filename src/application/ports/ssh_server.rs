use std::{future::Future, pin::Pin};

pub(crate) type SshServerFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send>>;
