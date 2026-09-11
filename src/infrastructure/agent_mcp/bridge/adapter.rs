use gpui_kit::{App, AppContext};

use crate::application::ApplicationContext;

use super::{dispatch, router::McpCommandRouter, types::McpBridgeReceiver};

pub(crate) fn start_mcp_bridge(cx: &mut App, bridge: McpBridgeReceiver) {
    let application = cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
    let McpBridgeReceiver {
        mut command_rx,
        notification_tx,
    } = bridge;

    let command_application = application.clone();
    cx.spawn(async move |_cx| {
        let mut router = McpCommandRouter::new(command_application);
        log::info!("MCP application bridge adapter started");
        while let Some(command) = command_rx.recv().await {
            if let Err(error) = router.route(command).await {
                log::debug!("MCP bridge command routing failed: {error}");
            }
        }
        log::info!(
            "MCP application bridge command router stopped: reason=ingress_closed, lanes={} ",
            router.lane_count()
        );
    })
    .detach();

    let notification_application = application;
    cx.spawn(async move |_cx| {
        let mut application_events = notification_application.subscribe();
        log::info!("MCP application notification forwarder started");
        loop {
            match application_events.recv().await {
                Ok(event) => {
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
    })
    .detach();
}
