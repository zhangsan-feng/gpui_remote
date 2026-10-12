use tokio::sync::{broadcast, watch};

use super::DataChange;

#[derive(Clone)]
pub(crate) struct DataContextNotice {
    session_lifecycle_events: broadcast::Sender<DataChange>,
    gui_refresh_notification: watch::Sender<u64>,
}

impl DataContextNotice {
    pub(super) fn new() -> Self {
        let (session_lifecycle_events, _) = broadcast::channel(256);
        let (gui_refresh_notification, _) = watch::channel(0);
        Self {
            session_lifecycle_events,
            gui_refresh_notification,
        }
    }

    pub(crate) fn subscribe_session_lifecycle(&self) -> broadcast::Receiver<DataChange> {
        self.session_lifecycle_events.subscribe()
    }

    pub(crate) fn subscribe_gui_refresh(&self) -> watch::Receiver<u64> {
        self.gui_refresh_notification.subscribe()
    }

    pub(super) fn notify_gui_refresh(&self) -> u64 {
        let mut sequence = 0;
        self.gui_refresh_notification.send_modify(|current| {
            *current = current.wrapping_add(1);
            sequence = *current;
        });
        sequence
    }

    pub(super) fn notify_session_lifecycle(&self, event: DataChange) {
        let _ = self.session_lifecycle_events.send(event);
    }
}
