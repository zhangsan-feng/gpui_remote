use crate::{
    application::{Application, ApplicationResult},
    event_bus::{EVENT_BUS, RefreshDataContext},
};

impl Application {
    pub(crate) fn start_event_bus(&self) {
        let mut task = self
            .data_refresh_task
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if task.is_some() {
            return;
        }
        let Some(mut requests) = EVENT_BUS.subscribe_refresh_data_context() else {
            return;
        };
        let application = self.clone();
        *task = Some(tokio::spawn(async move {
            let mut pending = tokio::task::JoinSet::new();
            log::info!("event_bus data refresh subscriber started");
            loop {
                tokio::select! {
                    request = requests.recv(), if pending.len() < 16 => {
                        let Some(request) = request else { break; };
                        let application = application.clone();
                        pending.spawn(async move {
                            // GUI callers retain run_application's operation guard until completion.
                            // Taking another read guard here can deadlock behind shutdown's writer.
                            let result = application.handle_data_refresh(request.event).await;
                            if let Err(error) = &result {
                                log::warn!("event_bus data refresh failed: {error}");
                            }
                            let _ = request.complete.send(result);
                        });
                    }
                    result = pending.join_next(), if !pending.is_empty() => {
                        if let Some(Err(error)) = result {
                            log::error!("event_bus data refresh task failed: {error}");
                        }
                    }
                }
            }
        }));
    }

    async fn handle_data_refresh(&self, event: RefreshDataContext) -> ApplicationResult<()> {
        tracing::debug!(?event, "event_bus data refresh received");
        match event {
            RefreshDataContext::NetworkConfigs => self.initialize_network_configs().await,
            RefreshDataContext::SftpDirectory {
                workspace_id,
                ip,
                title,
                path,
                local,
            } => {
                if local {
                    self.sftp
                        .change_local_directory_checked(workspace_id, ip, title, path)
                        .await
                } else {
                    self.sftp
                        .change_remote_directory_checked(workspace_id, ip, title, path)
                        .await
                }
            }
        }
    }

    pub(crate) async fn stop_event_bus(&self) {
        let task = self
            .data_refresh_task
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
            log::info!("event_bus data refresh subscriber stopped");
        }
    }
}
