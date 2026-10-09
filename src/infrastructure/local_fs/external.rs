use std::path::PathBuf;

use crate::{
    application::ports::{LocalEntry, LocalFsFuture, LocalWatchSource},
    domain::sftp::FileSignature,
    infrastructure::InfrastructureContext,
};

use super::core;

impl InfrastructureContext {
    pub(crate) fn default_directory(&self) -> LocalFsFuture<PathBuf> {
        Box::pin(async move {
            tokio::task::spawn_blocking(core::default_desktop_path)
                .await
                .map_err(Into::into)
        })
    }

    pub(crate) fn scan_directory(
        &self,
        path: PathBuf,
    ) -> LocalFsFuture<(PathBuf, Vec<LocalEntry>)> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || core::scan_local_directory(&path)).await?
        })
    }

    pub(crate) fn delete_path(&self, path: PathBuf) -> LocalFsFuture<()> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || core::delete_local_path(&path)).await?
        })
    }

    pub(crate) fn watch_path(&self, path: PathBuf) -> LocalFsFuture<LocalWatchSource> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || core::create_local_watcher(path)).await?
        })
    }

    pub(crate) fn signature(&self, path: PathBuf) -> LocalFsFuture<Option<FileSignature>> {
        Box::pin(async move {
            tokio::task::spawn_blocking(move || core::read_local_signature(&path)).await?
        })
    }
}
