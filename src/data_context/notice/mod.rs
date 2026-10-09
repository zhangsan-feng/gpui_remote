use tokio::sync::{broadcast, watch};

use super::{DataChange, DataContextNotice};

impl DataContextNotice {
    pub(super) fn new() -> Self {
        let (session_lifecycle_events, _) = broadcast::channel(256);
        let (gui_refresh_notification, _) = watch::channel(());
        Self {
            session_lifecycle_events,
            gui_refresh_notification,
        }
    }

    pub(crate) fn subscribe_session_lifecycle(&self) -> broadcast::Receiver<DataChange> {
        self.session_lifecycle_events.subscribe()
    }

    pub(crate) fn subscribe_gui_refresh(&self) -> watch::Receiver<()> {
        self.gui_refresh_notification.subscribe()
    }

    pub(super) fn notify_gui_refresh(&self) {
        self.gui_refresh_notification.send_replace(());
    }

    pub(super) fn notify_session_lifecycle(&self, event: DataChange) {
        let _ = self.session_lifecycle_events.send(event);
    }
}
