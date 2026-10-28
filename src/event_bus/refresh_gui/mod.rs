mod external;
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GuiComponent {
    Workspace,
    Terminal,
    Sftp,
    Services,
    NetworkConfig,
}

/// Independent, coalescing channels prevent terminal output from redrawing unrelated views.
#[derive(Default)]
pub(crate) struct RefreshGui {
    channels: Mutex<HashMap<GuiComponent, watch::Sender<u64>>>,
}
