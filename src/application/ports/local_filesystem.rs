use std::{future::Future, path::PathBuf, pin::Pin};

use anyhow::Result;
use tokio::sync::mpsc;

pub(crate) use crate::domain::sftp::LocalEntry;

pub(crate) type LocalFsFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

pub(crate) struct LocalWatchSource {
    pub(crate) events: mpsc::UnboundedReceiver<Result<Vec<PathBuf>, String>>,
    pub(crate) is_directory: bool,
    pub(crate) watcher: Box<dyn Send>,
}
