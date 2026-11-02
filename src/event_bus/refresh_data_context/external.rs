use super::*;

impl RefreshDataContextBus {
    /// GUI callers await this inside run_application so shutdown drains accepted IO.
    /// The subscriber must be started by Application before sending requests.
    pub(crate) async fn send(&self, event: RefreshDataContext) -> Result<(), String> {
        let (complete, result) = oneshot::channel();
        self.requests
            .send(DataRefreshRequest { event, complete })
            .await
            .map_err(|_| "数据刷新订阅已关闭".to_owned())?;
        result.await.map_err(|_| "数据刷新任务已停止".to_owned())?
    }

    pub(crate) fn subscribe(&self) -> Option<mpsc::Receiver<DataRefreshRequest>> {
        self.receiver
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    pub(crate) fn subscribe_session_lifecycle(&self) -> broadcast::Receiver<DataChange> {
        self.session_lifecycle.subscribe()
    }

    pub(crate) fn notify_session_lifecycle(&self, event: DataChange) {
        tracing::debug!(?event, "event_bus session lifecycle published");
        let _ = self.session_lifecycle.send(event);
    }
}
