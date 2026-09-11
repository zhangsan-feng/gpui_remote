use gpui_kit::{App, AppContext};
use std::time::Instant;

use crate::application::ApplicationContext;

use super::{dispatch, types::McpBridgeReceiver};

pub(crate) fn start_mcp_bridge(cx: &mut App, mut bridge: McpBridgeReceiver) {
    let application = cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
    let mut application_events = application.subscribe();
    let notification_tx = bridge.notification_tx.clone();

    cx.spawn(async move |_cx| {
        log::info!("MCP application bridge adapter started");
        loop {
            tokio::select! {
                command = bridge.command_rx.recv() => {
                    let Some(command) = command else {
                        log::info!("MCP application bridge command channel closed");
                        break;
                    };
                    let request_id = command.request_id.clone();
                    let command_name = command.command.name();
                    let workspace_id = command.command.workspace_id().unwrap_or("control").to_owned();
                    let queue_ms = command.queued_at.elapsed().as_millis();
                    log::debug!(
                        "MCP bridge command received: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, queue_ms={queue_ms}"
                    );
                    let dispatch_started = Instant::now();
                    let result = dispatch::dispatch(&application, command.command).await;
                    let application_ms = dispatch_started.elapsed().as_millis();
                    let total_ms = command.queued_at.elapsed().as_millis();
                    let application_failed = result.is_err();
                    if application_failed {
                        log::warn!(
                            "MCP bridge application error: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}"
                        );
                    }
                    if command.response_tx.send(super::types::ResponseEnvelope { request_id: request_id.clone(), result }).is_err() {
                        log::debug!("MCP bridge response receiver dropped: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}");
                    } else {
                        log::debug!("MCP bridge response sent: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, queue_ms={queue_ms}, application_ms={application_ms}, total_ms={total_ms}");
                    }
                }
                event = application_events.recv() => {
                    match event {
                        Ok(event) => {
                            let _ = notification_tx.send(dispatch::map_event(event));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            log::warn!("MCP bridge notification receiver lagged: skipped={count}");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            application_events = application.subscribe();
                        }
                    }
                }
            }
        }
        log::info!("MCP application bridge adapter stopped");
    })
    .detach();
}
