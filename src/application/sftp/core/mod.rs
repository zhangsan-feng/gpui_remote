mod remote;
mod service;
mod watcher;

use std::path::PathBuf;

use tokio::sync::oneshot;

pub(crate) use crate::application::ports::RemoteDeleteItem;
pub(crate) use crate::data_context::{LocalSnapshot, SftpModel, TransferRecord, TransferRequest};
pub(crate) use service::SftpApplication;
pub(crate) use watcher::{LocalWatchRuntime, LocalWatchSummary};

enum SftpCommand {
    ChangeRemoteDirectory {
        path: String,
        complete: oneshot::Sender<Result<String, String>>,
    },
    Upload {
        transfer_id: u64,
        local_path: PathBuf,
        remote_path: String,
        refresh_path: String,
        complete: Option<oneshot::Sender<bool>>,
    },
    Download {
        transfer_id: u64,
        remote_path: String,
        local_path: PathBuf,
        total_size: u64,
        is_directory: bool,
        complete: oneshot::Sender<bool>,
    },
    Delete {
        items: Vec<RemoteDeleteItem>,
        refresh_path: String,
    },
    Disconnect,
}
