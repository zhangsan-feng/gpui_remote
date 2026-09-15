use gpui_kit::{App, AppContext};
use std::time::Duration;

use crate::application::{ApplicationContext, ApplicationEvent};

use super::{dispatch, router::McpCommandRouter, types::McpBridgeReceiver};

pub(crate) fn start_mcp_bridge(cx: &App, bridge: McpBridgeReceiver) {
    let application = cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
    let McpBridgeReceiver {
        mut command_rx,
        notification_tx,
    } = bridge;

    let command_application = application.clone();
    // This bridge owns Tokio channels and timers. Keep it on the Tokio runtime
    // instead of GPUI's foreground executor, whose wake-up model can strand a
    // pending Tokio channel operation under load.
    tokio::spawn(async move {
        let lifecycle_application = command_application.clone();
        let mut router = McpCommandRouter::new(command_application);
        let mut application_events = lifecycle_application.subscribe();
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
                event = application_events.recv() => {
                    match event {
                        Ok(event) => {
                            log::debug!(
                                "MCP bridge application event observed: event={}, workspace_id={}",
                                application_event_name(&event),
                                application_event_workspace_id(&event).unwrap_or("none")
                            );
                            if let ApplicationEvent::SessionClosed { workspace_id } = event {
                            router.remove(&workspace_id);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            log::warn!("MCP bridge lifecycle receiver lagged: skipped={count}");
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
            "MCP application bridge command router stopped: reason=ingress_closed, lanes={} ",
            router.lane_count()
        );
    });

    let notification_application = application;
    tokio::spawn(async move {
        let mut application_events = notification_application.subscribe();
        log::info!("MCP application notification forwarder started");
        loop {
            match application_events.recv().await {
                Ok(event) => {
                    log::debug!(
                        "MCP application notification forwarded: event={}, workspace_id={}",
                        application_event_name(&event),
                        application_event_workspace_id(&event).unwrap_or("none")
                    );
                    let _ = notification_tx.send(dispatch::map_event(event));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    log::warn!("MCP bridge notification receiver lagged: skipped={count}");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    log::info!(
                        "MCP application notification forwarder stopped: reason=application_events_closed"
                    );
                    break;
                }
            }
        }
    });
}

fn application_event_name(event: &ApplicationEvent) -> &'static str {
    match event {
        ApplicationEvent::SessionOpened { .. } => "session_opened",
        ApplicationEvent::SessionClosed { .. } => "session_closed",
        ApplicationEvent::SessionSelected { .. } => "session_selected",
    }
}

fn application_event_workspace_id(event: &ApplicationEvent) -> Option<&str> {
    match event {
        ApplicationEvent::SessionOpened { workspace_id, .. }
        | ApplicationEvent::SessionClosed { workspace_id } => Some(workspace_id),
        ApplicationEvent::SessionSelected { workspace_id } => workspace_id.as_deref(),
    }
}
