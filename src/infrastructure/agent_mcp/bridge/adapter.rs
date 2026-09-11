use gpui_kit::{App, AppContext};

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
                    log::debug!("MCP bridge command received: request_id={request_id}");
                    let result = dispatch::dispatch(&application, command.command).await;
                    if command.response_tx.send(super::types::ResponseEnvelope { request_id: request_id.clone(), result }).is_err() {
                        log::debug!("MCP bridge response receiver dropped: request_id={request_id}");
                    } else {
                        log::debug!("MCP bridge response sent: request_id={request_id}");
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
