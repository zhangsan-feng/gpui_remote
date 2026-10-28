mod external;
pub(crate) mod refresh_gui;

use crate::data_context::DataChange;
use std::sync::{LazyLock, Mutex};
use tokio::sync::{broadcast, mpsc, oneshot};

pub(crate) static EVENT_BUS: LazyLock<EventBus> = LazyLock::new(EventBus::new);

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

pub(crate) struct EventBus {
    pub(crate) refresh_gui: refresh_gui::RefreshGui,
    session_lifecycle: broadcast::Sender<DataChange>,
    refresh_data: mpsc::Sender<DataRefreshRequest>,
    refresh_receiver: Mutex<Option<mpsc::Receiver<DataRefreshRequest>>>,
}

impl EventBus {
    fn new() -> Self {
        let (refresh_data, receiver) = mpsc::channel(64);
        Self {
            refresh_gui: Default::default(),
            session_lifecycle: broadcast::channel(256).0,
            refresh_data,
            refresh_receiver: Mutex::new(Some(receiver)),
        }
    }
}
