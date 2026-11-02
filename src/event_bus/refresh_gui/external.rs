use super::*;

impl RefreshGui {
    pub(crate) fn subscribe(&self, component: GuiComponent) -> GuiRefreshReceiver {
        let (sender, revision) = watch::channel(0);
        let mailbox = Arc::new(RefreshMailbox {
            pending: Mutex::new(RefreshScope::default()),
            revision: sender,
        });
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        let subscribers = channels.entry(component).or_default();
        subscribers.retain(|subscriber| subscriber.strong_count() > 0);
        subscribers.push(Arc::downgrade(&mailbox));
        GuiRefreshReceiver { mailbox, revision }
    }

    pub(crate) fn notify(&self, components: &[GuiComponent]) -> u64 {
        self.dispatch(components, None)
    }

    pub(crate) fn notify_workspace(&self, components: &[GuiComponent], workspace_id: &str) -> u64 {
        self.dispatch(components, Some(workspace_id))
    }

    fn dispatch(&self, components: &[GuiComponent], workspace_id: Option<&str>) -> u64 {
        let mut channels = self.channels.lock().unwrap_or_else(|e| e.into_inner());
        let mut sequence = 0;
        for component in components {
            let Some(subscribers) = channels.get_mut(component) else {
                continue;
            };
            subscribers.retain(|subscriber| {
                let Some(mailbox) = subscriber.upgrade() else {
                    return false;
                };
                let mut pending = mailbox.pending.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(id) = workspace_id {
                    if !pending.all {
                        pending.workspace_ids.insert(id.to_owned());
                    }
                } else {
                    pending.all = true;
                    pending.workspace_ids.clear();
                }
                mailbox.revision.send_modify(|revision| {
                    *revision = revision.wrapping_add(1);
                    sequence = *revision;
                });
                true
            });
        }
        tracing::debug!(?components, workspace_id, "event_bus GUI refresh requested");
        sequence
    }
}

impl GuiRefreshReceiver {
    pub(crate) async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.revision.changed().await
    }

    /// Mark the wakeup consumed and drain this subscriber's IDs under the same lock.
    pub(crate) fn take_pending(&mut self) -> RefreshScope {
        let mut pending = self
            .mailbox
            .pending
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.revision.borrow_and_update();
        std::mem::take(&mut *pending)
    }
}
