use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Instant};

use serde::Serialize;

use crate::domain::{
    sftp::{LocalEntry, SftpEntry},
    terminal::TerminalStatus,
};

pub(crate) fn is_transfer_cancellable(status: &str) -> bool {
    matches!(status, "排队中" | "扫描中" | "传输中")
}

#[derive(Clone, Debug)]
pub(crate) struct SftpSnapshot {
    pub(crate) status: TerminalStatus,
    pub(crate) path: String,
    pub(crate) entries: Arc<Vec<SftpEntry>>,
    pub(crate) loading: bool,
    pub(crate) error: Option<String>,
}

impl Default for SftpSnapshot {
    fn default() -> Self {
        Self {
            status: TerminalStatus::Connecting,
            path: String::new(),
            entries: Arc::new(Vec::new()),
            loading: true,
            error: None,
        }
    }
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
            path: PathBuf::new(),
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

#[derive(Clone)]
pub(in crate::data_context) struct SftpWorkspaceData {
    pub(in crate::data_context) remote: SftpSnapshot,
    pub(in crate::data_context) local: LocalSnapshot,
    pub(in crate::data_context) local_scan_generation: u64,
    pub(in crate::data_context) local_requested_path: Option<PathBuf>,
    pub(in crate::data_context) remote_navigation_generation: u64,
    pub(in crate::data_context) transfers: Vec<TransferRecord>,
    pub(in crate::data_context) watches: Vec<SftpWatchSummary>,
    pub(in crate::data_context) cancelled_transfers: HashSet<u64>,
    pub(in crate::data_context) last_transfer_notification: Option<Instant>,
    pub(in crate::data_context) remote_revision: u64,
}

impl Default for SftpWorkspaceData {
    fn default() -> Self {
        Self {
            remote: SftpSnapshot::default(),
            local: LocalSnapshot::default(),
            local_scan_generation: 0,
            local_requested_path: None,
            remote_navigation_generation: 0,
            transfers: Vec::new(),
            watches: Vec::new(),
            cancelled_transfers: HashSet::new(),
            last_transfer_notification: None,
            remote_revision: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpEntrySummary {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SftpDirectorySummary {
    pub path: String,
    pub entries: Vec<SftpEntrySummary>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpTransferSummary {
    pub queued: usize,
    pub transfers: Vec<SftpTransferInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpTransferInfo {
    pub id: u64,
    pub workspace_id: String,
    pub name: String,
    pub direction: String,
    pub source: String,
    pub target: String,
    pub is_directory: bool,
    pub progress: f32,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_second: u64,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpWatchSummary {
    pub workspace_id: String,
    pub ip: String,
    pub title: String,
    pub local_path: String,
    pub remote_path: String,
    pub is_directory: bool,
    pub debounce_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpWorkspaceSnapshot {
    pub remote: SftpDirectorySummary,
    pub local: SftpDirectorySummary,
    pub transfers: Vec<SftpTransferInfo>,
    pub watches: Vec<SftpWatchSummary>,
    pub status: String,
    pub remote_revision: u64,
}

impl Default for SftpWorkspaceSnapshot {
    fn default() -> Self {
        Self {
            remote: SftpDirectorySummary {
                loading: true,
                ..SftpDirectorySummary::default()
            },
            local: SftpDirectorySummary {
                loading: true,
                ..SftpDirectorySummary::default()
            },
            transfers: Vec::new(),
            watches: Vec::new(),
            status: "connecting".to_owned(),
            remote_revision: 0,
        }
    }
}
