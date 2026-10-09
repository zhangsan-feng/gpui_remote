mod core;
mod external;
mod relay;

use std::sync::{Arc, atomic::AtomicU64};

use tokio::{
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
};

use crate::domain::http_proxy::{HttpProxyConnection, HttpProxyStatus};

pub(super) enum ConnectionEvent {
    Opened(HttpProxyConnection),
    Closed(u64),
}

struct RunningService {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
    ready: Option<oneshot::Sender<()>>,
}

#[derive(Default)]
struct RuntimeState {
    running: Option<RunningService>,
}

pub(crate) struct HttpProxyController {
    operation: Arc<Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
    generation: Arc<AtomicU64>,
    status: watch::Sender<HttpProxyStatus>,
}

impl HttpProxyController {
    pub(crate) fn new() -> Self {
        let (status, _) = watch::channel(HttpProxyStatus::default());
        Self {
            operation: Arc::new(Mutex::new(())),
            state: Arc::new(Mutex::new(RuntimeState::default())),
            generation: Arc::new(AtomicU64::new(0)),
            status,
        }
    }
}

pub(crate) type HttpProxyFuture<T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, String>> + Send>>;
