mod core;
mod external;
pub(crate) mod model;

use crate::domain::port_forward::PortForwardStatus;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

pub(crate) use model::PortForwardRuleDraft;

#[derive(Default)]
struct PortForwardProjection {
    statuses: HashMap<String, PortForwardStatus>,
    runtime_ids: HashSet<String>,
}

#[derive(Clone)]
pub(crate) struct PortForwardApplication {
    mutations: Arc<AsyncMutex<()>>,
    statuses: Arc<Mutex<PortForwardProjection>>,
    subscription: Arc<AsyncMutex<Option<JoinHandle<()>>>>,
}

impl PortForwardApplication {
    pub(crate) fn new() -> Self {
        Self {
            mutations: Arc::new(AsyncMutex::new(())),
            statuses: Arc::new(Mutex::new(PortForwardProjection::default())),
            subscription: Arc::new(AsyncMutex::new(None)),
        }
    }
}
