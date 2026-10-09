use std::{future::Future, path::Path, pin::Pin};

use anyhow::Result;

pub(crate) use crate::domain::sftp::SftpEntry;

pub(crate) type SftpFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;
pub(crate) type SftpConnectFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn SftpConnection>>> + Send + 'a>>;
pub(crate) type SftpTransferFuture<'a> = SftpFuture<'a, SftpTransferOutcome>;
pub(crate) type SftpProgressCallback<'a> = Box<dyn FnMut(u64, u64) + Send + 'a>;
pub(crate) type SftpCancellationCallback<'a> = Box<dyn Fn() -> bool + Send + Sync + 'a>;

pub(crate) trait SftpConnection: Send + Sync {
    fn canonicalize<'a>(&'a self, path: &'a str) -> SftpFuture<'a, String>;
    fn read_directory<'a>(&'a self, path: &'a str) -> SftpFuture<'a, Vec<SftpEntry>>;
    fn delete_paths<'a>(&'a self, items: &'a [RemoteDeleteItem]) -> SftpFuture<'a, ()>;
    fn upload_path<'a>(
        &'a self,
        local_path: &'a Path,
        remote_path: &'a str,
        on_progress: SftpProgressCallback<'a>,
        is_cancelled: SftpCancellationCallback<'a>,
    ) -> SftpTransferFuture<'a>;
    fn download_path<'a>(
        &'a self,
        remote_path: &'a str,
        local_path: &'a Path,
        is_directory: bool,
        on_progress: SftpProgressCallback<'a>,
        is_cancelled: SftpCancellationCallback<'a>,
    ) -> SftpTransferFuture<'a>;
    fn close(&self) -> SftpFuture<'_, ()>;
}

#[derive(Clone, Debug)]
pub(crate) struct RemoteDeleteItem {
    pub(crate) path: String,
    pub(crate) is_directory: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SftpTransferOutcome {
    pub(crate) is_directory: bool,
    pub(crate) total_size: u64,
    pub(crate) transferred_files: u64,
    pub(crate) skipped_files: u64,
    pub(crate) unchanged_entries: u64,
    pub(crate) transferred_bytes: u64,
}
