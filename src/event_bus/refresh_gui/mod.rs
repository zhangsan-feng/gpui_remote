mod external;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GuiComponent {
    Workspace,
    Terminal,
    HostInfo,
    Sftp,
    Services,
    NetworkConfig,
}

/// Pending IDs are merged per subscriber, so bursts and multiple windows lose no sessions.
#[derive(Default)]
pub(crate) struct RefreshScope {
    pub(crate) all: bool,
    pub(crate) workspace_ids: HashSet<String>,
}

struct RefreshMailbox {
    pending: Mutex<RefreshScope>,
    revision: watch::Sender<u64>,
}

pub(crate) struct GuiRefreshReceiver {
    mailbox: Arc<RefreshMailbox>,
    revision: watch::Receiver<u64>,
}

#[derive(Default)]
pub(crate) struct RefreshGui {
    channels: Mutex<HashMap<GuiComponent, Vec<Weak<RefreshMailbox>>>>,
}
