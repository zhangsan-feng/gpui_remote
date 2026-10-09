mod core;
mod external;
mod relay;

use std::{collections::HashMap, sync::Arc};

use tokio::{
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
};

use crate::domain::port_forward::PortForwardStatus;

type StatusSnapshot = HashMap<String, PortForwardStatus>;

struct RunningForward {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

pub(crate) struct PortForwardController {
    running: Mutex<HashMap<String, RunningForward>>,
    persistence: Mutex<()>,
    status: watch::Sender<StatusSnapshot>,
}

impl PortForwardController {
    pub(crate) fn new() -> Arc<Self> {
        let (status, _) = watch::channel(HashMap::new());
        Arc::new(Self {
            running: Mutex::new(HashMap::new()),
            persistence: Mutex::new(()),
            status,
        })
    }
}
