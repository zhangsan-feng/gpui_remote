use super::*;

impl GuiEventBus {
    pub(crate) fn subscribe(&self, scope: EntityId, target: GuiTarget) -> GuiEventSubscription {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.mailboxes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((scope, target), sender.clone());
        GuiEventSubscription {
            scope,
            target,
            sender,
            receiver,
        }
    }

    pub(crate) fn send(&self, scope: EntityId, event: GuiEvent) {
        let target = event.target();
        let mut mailboxes = self.mailboxes.lock().unwrap_or_else(|e| e.into_inner());
        if !mailboxes
            .get(&(scope, target))
            .is_some_and(|sender| sender.send(event).is_ok())
        {
            mailboxes.remove(&(scope, target));
            tracing::warn!(?scope, ?target, "GUI event has no active recipient");
        }
    }

    /// Shared profiles and theme settings affect every main window.
    pub(crate) fn broadcast(&self, event: GuiEvent) {
        let target = event.target();
        let mut mailboxes = self.mailboxes.lock().unwrap_or_else(|e| e.into_inner());
        mailboxes.retain(|(_, recipient), sender| {
            *recipient != target || sender.send(event.clone()).is_ok()
        });
    }
}
