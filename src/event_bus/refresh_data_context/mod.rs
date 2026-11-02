mod external;
use crate::data_context::WorkspaceSummary;
use std::sync::Mutex;
use tokio::sync::{broadcast, mpsc, oneshot};

/// Requests IO through Application; GUI notifications are sent only after snapshots commit.
#[derive(Debug)]
pub(crate) enum RefreshDataContext {
    NetworkConfigs,
    SftpDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        local: bool,
    },
}

pub(crate) struct DataRefreshRequest {
    pub(crate) event: RefreshDataContext,
    pub(crate) complete: oneshot::Sender<Result<(), String>>,
}

#[derive(Clone, Debug)]
pub(crate) enum DataChange {
    SessionOpened {
        workspace_id: String,
        profile: WorkspaceSummary,
    },
    SessionClosed {
        workspace_id: String,
    },
    SessionSelected {
        workspace_id: Option<String>,
    },
}

/// Requests are consumed by Application; committed lifecycle events fan out to MCP.
pub(crate) struct RefreshDataContextBus {
    requests: mpsc::Sender<DataRefreshRequest>,
    receiver: Mutex<Option<mpsc::Receiver<DataRefreshRequest>>>,
    session_lifecycle: broadcast::Sender<DataChange>,
}

impl Default for RefreshDataContextBus {
    fn default() -> Self {
        let (requests, receiver) = mpsc::channel(64);
        Self {
            requests,
            receiver: Mutex::new(Some(receiver)),
            session_lifecycle: broadcast::channel(256).0,
        }
    }
}
