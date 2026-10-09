mod notice;
mod service_status;
mod sftp;
mod state;
mod terminal;
mod workspace;

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex, RwLock, atomic::AtomicU64},
};
use tokio::sync::{broadcast, watch};

pub(crate) static DATA_CONTEXT: LazyLock<DataContext> = LazyLock::new(DataContext::new);

pub(crate) struct DataContext {
    snapshot: RwLock<DataSnapshot>,
    terminals: RwLock<HashMap<String, state::TerminalWorkspaceData>>,
    sftp: RwLock<HashMap<String, state::SftpWorkspaceData>>,
    next_sftp_transfer_id: AtomicU64,
    commit: Mutex<()>,
    pub(crate) notice: DataContextNotice,
}

#[derive(Clone)]
pub(crate) struct DataContextNotice {
    session_lifecycle_events: broadcast::Sender<DataChange>,
    gui_refresh_notification: watch::Sender<()>,
}

impl DataContext {
    fn new() -> Self {
        Self {
            snapshot: RwLock::new(DataSnapshot::default()),
            terminals: RwLock::new(HashMap::new()),
            sftp: RwLock::new(HashMap::new()),
            next_sftp_transfer_id: AtomicU64::new(1),
            commit: Mutex::new(()),
            notice: DataContextNotice::new(),
        }
    }
}

pub(crate) use sftp::SftpModel;
pub(crate) use state::{
    DataChange, DataSnapshot, LocalSnapshot, SftpDirectorySummary, SftpEntrySummary, SftpSnapshot,
    SftpTransferInfo, SftpTransferSummary, SftpWatchSummary, SftpWorkspaceSnapshot,
    TerminalReadSnapshot, TransferRecord, TransferRequest, WorkspaceSummary,
    is_transfer_cancellable,
};
