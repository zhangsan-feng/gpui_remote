use std::collections::HashSet;
use std::time::Duration;

use crate::{application::Application, data_context::DataChange};

use super::{
    dispatch,
    router::McpCommandRouter,
    types::{ApplicationNotification, McpBridgeReceiver, NotificationEnvelope},
};

pub(crate) fn start_mcp_bridge(
    application: Application,
    bridge: McpBridgeReceiver,
) -> Result<(), String> {
    let McpBridgeReceiver {
        mut command_rx,
        notification_tx,
        data_context,
        notice,
    } = bridge;

    let lifecycle_data_context = data_context;
    let notification_data_context = data_context;
    // Subscribe synchronously before serving commands so early events are observed.
    let mut lifecycle_events = notice.subscribe_session_lifecycle();
    let mut notification_events = notice.subscribe_session_lifecycle();
    // This bridge owns Tokio channels and timers. Keep it on the Tokio runtime
    // instead of GPUI's foreground executor, whose wake-up model can strand a
    // pending Tokio channel operation under load.
    tokio::spawn(async move {
        let mut router = McpCommandRouter::new(application);
        let mut idle_cleanup = tokio::time::interval(Duration::from_secs(60));
        log::info!("MCP application bridge adapter started");
        loop {
            tokio::select! {
                command = command_rx.recv() => {
                    let Some(command) = command else {
                        log::info!("MCP application bridge command channel closed");
                        break;
                    };
                    let request_id = command.request_id.clone();
                    let command_name = command.command.name();
                    let workspace_id = command
                        .command
                        .workspace_id()
                        .unwrap_or("control")
                        .to_owned();
                    log::debug!(
                        "MCP bridge ingress received: request_id={request_id}, command={command_name}, workspace_id={workspace_id}"
                    );
                    let route_started = std::time::Instant::now();
                    if let Err(error) = router.route(command).await {
                        log::debug!("MCP bridge command routing failed: {error}");
                    }
                    log::debug!(
                        "MCP bridge ingress routed: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, route_ms={}",
                        route_started.elapsed().as_millis()
                    );
                }
                event = lifecycle_events.recv() => {
                    match event {
                        Ok(event) => {
                            log::debug!(
                                "MCP bridge data context event observed: event={}, workspace_id={}",
                                context_event_name(&event),
                                context_event_workspace_id(&event).unwrap_or("none")
                            );
                            if let DataChange::SessionClosed { workspace_id } = event {
                                router.remove(&workspace_id);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            log::warn!("MCP bridge lifecycle receiver lagged: skipped={count}");
                            let open_workspaces = lifecycle_data_context
                                .workspace_snapshot()
                                .workspaces
                                .into_iter()
                                .map(|workspace| workspace.workspace_id)
                                .collect::<HashSet<_>>();
                            router.retain_open_workspaces(&open_workspaces);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            log::warn!(
                                "MCP bridge lifecycle receiver closed: command router stopped"
                            );
                            break;
                        }
                    }
                }
                _ = idle_cleanup.tick() => {
                    router.cleanup_idle();
                }
            }
        }
        log::info!(
            "MCP application bridge command router stopped: reason=ingress_closed, lanes={}",
            router.lane_count()
        );
    });

    tokio::spawn(async move {
        log::info!("MCP data context notification forwarder started");
        loop {
            match notification_events.recv().await {
                Ok(event) => {
                    log::debug!(
                        "MCP data context notification forwarded: event={}, workspace_id={}",
                        context_event_name(&event),
                        context_event_workspace_id(&event).unwrap_or("none")
                    );
                    let _ = notification_tx.send(dispatch::map_event(event));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!("MCP bridge notification receiver lagged: skipped={count}");
                    let snapshot = notification_data_context.workspace_snapshot();
                    log::info!(
                        "MCP bridge notification snapshot refreshed: revision={}, open_workspaces={}",
                        snapshot.revision,
                        snapshot.workspaces.len()
                    );
                    let _ = notification_tx.send(NotificationEnvelope {
                        event: ApplicationNotification::ResyncRequired,
                    });
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    log::info!(
                        "MCP data context notification forwarder stopped: reason=context_events_closed"
                    );
                    break;
                }
            }
        }
    });

    Ok(())
}

fn context_event_name(event: &DataChange) -> &'static str {
    match event {
        DataChange::SessionOpened { .. } => "session_opened",
        DataChange::SessionClosed { .. } => "session_closed",
        DataChange::SessionSelected { .. } => "session_selected",
    }
}

fn context_event_workspace_id(event: &DataChange) -> Option<&str> {
    match event {
        DataChange::SessionOpened { workspace_id, .. }
        | DataChange::SessionClosed { workspace_id } => Some(workspace_id),
        DataChange::SessionSelected { workspace_id } => workspace_id.as_deref(),
    }
}
