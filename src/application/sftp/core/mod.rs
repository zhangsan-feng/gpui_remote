mod conn;
mod local;
mod path;
mod remote;
mod service;
mod sync;
mod transfer;
mod watcher;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant, SystemTime},
};

use tokio::sync::{Notify, oneshot};

pub(crate) use path::default_desktop_path;
pub(crate) use service::SftpApplication;
pub(crate) use watcher::{LocalWatchRuntime, LocalWatchSummary};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SftpStatus {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Clone, Debug)]
pub(crate) struct SftpEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_directory: bool,
    pub(crate) size: u64,
    pub(crate) modified_at: Option<u32>,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalEntry {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_directory: bool,
    pub(crate) size: u64,
    pub(crate) modified_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalSnapshot {
    pub(crate) path: PathBuf,
    pub(crate) entries: Arc<Vec<LocalEntry>>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
}

impl Default for LocalSnapshot {
    fn default() -> Self {
        Self {
            path: default_desktop_path(),
            entries: Arc::new(Vec::new()),
            loading: true,
            error: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransferRecord {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) direction: String,
    pub(crate) target: String,
    pub(crate) request: TransferRequest,
    pub(crate) progress: f32,
    pub(crate) transferred_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) speed: u64,
    pub(crate) started_at: Option<Instant>,
    pub(crate) speed_updated_at: Option<Instant>,
    pub(crate) status: String,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum TransferRequest {
    Upload {
        workspace_id: String,
        local_path: PathBuf,
        is_directory: bool,
    },
    Download {
        workspace_id: String,
        remote_path: String,
        file_name: String,
        total_size: u64,
        is_directory: bool,
    },
}

impl TransferRequest {
    pub(crate) fn workspace_id(&self) -> &str {
        match self {
            Self::Upload { workspace_id, .. } | Self::Download { workspace_id, .. } => workspace_id,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SftpSnapshot {
    pub(crate) status: SftpStatus,
    pub(crate) path: String,
    pub(crate) entries: Arc<Vec<SftpEntry>>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
}

impl Default for SftpSnapshot {
    fn default() -> Self {
        Self {
            status: SftpStatus::Connecting,
            path: String::new(),
            entries: Arc::new(Vec::new()),
            loading: true,
            error: None,
        }
    }
}

struct SftpModel {
    snapshot: RwLock<SftpSnapshot>,
    revision: std::sync::atomic::AtomicU64,
    transfers: Arc<RwLock<Vec<TransferRecord>>>,
    cancelled_transfers: RwLock<HashSet<u64>>,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
    transfer_ui_throttle: Arc<Mutex<Option<Instant>>>,
}

impl SftpModel {
    fn new(
        transfers: Arc<RwLock<Vec<TransferRecord>>>,
        updates: Arc<Notify>,
        status_updates: Arc<Notify>,
        transfer_ui_throttle: Arc<Mutex<Option<Instant>>>,
    ) -> Self {
        Self {
            snapshot: RwLock::new(SftpSnapshot::default()),
            revision: std::sync::atomic::AtomicU64::new(0),
            transfers,
            cancelled_transfers: RwLock::new(HashSet::new()),
            updates,
            status_updates,
            transfer_ui_throttle,
        }
    }

    pub(crate) fn snapshot(&self) -> SftpSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(std::sync::atomic::Ordering::Acquire)
    }

    fn update(&self, update: impl FnOnce(&mut SftpSnapshot), status_changed: bool) {
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            update(&mut snapshot);
        }
        self.revision
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.updates.notify_one();
        if status_changed {
            self.status_updates.notify_waiters();
        }
    }

    fn set_connected(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(
            move |snapshot| {
                snapshot.status = SftpStatus::Connected;
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            true,
        );
    }

    fn set_loading(&self) {
        self.update(
            |snapshot| {
                snapshot.loading = true;
                snapshot.error = None;
            },
            false,
        );
    }

    fn set_directory(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(
            move |snapshot| {
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            false,
        );
    }

    fn set_error(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            false,
        );
    }

    pub(crate) fn set_failed(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.status = SftpStatus::Failed;
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            true,
        );
    }

    fn update_transfer(
        &self,
        transfer_id: u64,
        progress: f32,
        transferred: u64,
        total: u64,
        status: impl Into<String>,
    ) {
        let status = status.into();
        let is_progress = status == "传输中" && transferred > 0;
        let now = Instant::now();
        let mut transfers = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(transfer) = transfers
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.progress = progress.clamp(0., 1.);
            if status == "扫描中" {
                transfer.speed = 0;
                transfer.transferred_bytes = 0;
                transfer.total_bytes = 0;
                transfer.error = None;
                transfer.started_at = None;
                transfer.speed_updated_at = None;
            } else if transferred > 0 {
                transfer.transferred_bytes = transferred;
                transfer.total_bytes = total;
                let started_at = transfer.started_at.get_or_insert(now);
                let elapsed = now.duration_since(*started_at).as_secs_f64();
                let should_update = transfer.speed_updated_at.is_none_or(|updated_at| {
                    now.duration_since(updated_at) >= Duration::from_secs(1)
                });
                if elapsed > 0. && should_update {
                    transfer.speed = (transferred as f64 / elapsed) as u64;
                    transfer.speed_updated_at = Some(now);
                }
            }
            transfer.status = status;
        }
        drop(transfers);

        let should_notify = if is_progress {
            let mut last_notify = self
                .transfer_ui_throttle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let should_notify = last_notify
                .is_none_or(|updated_at| now.duration_since(updated_at) >= Duration::from_secs(1));
            if should_notify {
                *last_notify = Some(now);
            }
            should_notify
        } else {
            true
        };
        if should_notify {
            self.updates.notify_one();
        }
    }

    pub(crate) fn request_cancel(&self, transfer_id: u64) {
        self.cancelled_transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(transfer_id);
        self.set_transfer_status(transfer_id, "已取消");
    }

    pub(crate) fn is_cancelled(&self, transfer_id: u64) -> bool {
        self.cancelled_transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&transfer_id)
    }

    fn set_transfer_status(&self, transfer_id: u64, status: &str) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.status = status.to_owned();
            if status == "已取消" {
                transfer.speed = 0;
            }
        }
        self.updates.notify_one();
    }

    pub(crate) fn set_transfer_error(&self, transfer_id: u64, error: String) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            transfer.status = "失败".to_owned();
            transfer.error = Some(error);
            transfer.speed = 0;
        }
        self.updates.notify_one();
    }

    fn set_upload_directory(&self, transfer_id: u64, is_directory: bool) {
        if let Some(transfer) = self
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter_mut()
            .find(|transfer| transfer.id == transfer_id)
        {
            if let TransferRequest::Upload {
                is_directory: current,
                ..
            } = &mut transfer.request
            {
                *current = is_directory;
            }
        }
        self.updates.notify_one();
    }
}

enum SftpCommand {
    ChangeRemoteDirectory(String),
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

#[derive(Clone, Debug)]
pub(crate) struct RemoteDeleteItem {
    pub(crate) path: String,
    pub(crate) is_directory: bool,
}

pub(crate) fn scan_local_directory(path: &Path) -> anyhow::Result<(PathBuf, Vec<LocalEntry>)> {
    local::scan_local_directory(path)
}

pub(crate) fn delete_local_path(path: &Path) -> anyhow::Result<()> {
    local::delete_local_path(path)
}
