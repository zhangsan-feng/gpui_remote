use gpui_kit::{App, AppContext};

use crate::{
    application::{ApplicationContext, ApplicationEvent, ApplicationResult},
    infrastructure::agent_mcp::bridge::{
        ApplicationCommand, ApplicationNotification, ApplicationResponse, McpBridgeReceiver,
        NotificationEnvelope, ResponseEnvelope,
    },
};

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
                    let result = dispatch(&application, command.command).await;
                    if command.response_tx.send(ResponseEnvelope { request_id: request_id.clone(), result }).is_err() {
                        log::debug!("MCP bridge response receiver dropped: request_id={request_id}");
                    } else {
                        log::debug!("MCP bridge response sent: request_id={request_id}");
                    }
                }
                event = application_events.recv() => {
                    match event {
                        Ok(event) => {
                            if let Some(notification) = map_event(event) {
                                let _ = notification_tx.send(notification);
                            }
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

async fn dispatch(
    application: &ApplicationContext,
    command: ApplicationCommand,
) -> ApplicationResult<ApplicationResponse> {
    match command {
        ApplicationCommand::ListProfiles => application
            .list_profiles()
            .await
            .map(ApplicationResponse::Profiles),
        ApplicationCommand::OpenSession {
            profile_id,
            protocol,
            ip,
            title,
        } => application
            .open_session(profile_id, protocol, ip, title)
            .await
            .map(ApplicationResponse::WorkspaceId),
        ApplicationCommand::ListSftpSessions => application
            .list_sftp_sessions()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::ListSftpLocal => application
            .list_sftp_local()
            .await
            .map(ApplicationResponse::SftpDirectory),
        ApplicationCommand::ChangeSftpLocalDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .change_sftp_local_directory(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpRemote { workspace_id } => application
            .list_sftp_remote(workspace_id)
            .await
            .map(ApplicationResponse::SftpDirectory),
        ApplicationCommand::ChangeSftpRemoteDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .change_sftp_remote_directory(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::UploadSftp {
            workspace_id,
            local_paths,
        } => application
            .upload_sftp(workspace_id, local_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::DownloadSftp {
            workspace_id,
            remote_paths,
        } => application
            .download_sftp(workspace_id, remote_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::ListSftpTransfers { workspace_id } => application
            .list_sftp_transfers(workspace_id)
            .await
            .map(ApplicationResponse::SftpTransferInfos),
        ApplicationCommand::WatchSftpLocal {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .watch_sftp_local(workspace_id, ip, title, local_path)
            .await
            .map(ApplicationResponse::SftpWatch),
        ApplicationCommand::StopSftpLocalWatch {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .stop_sftp_local_watch(workspace_id, ip, title, local_path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpLocalWatches {
            workspace_id,
            ip,
            title,
        } => application
            .list_sftp_local_watches(workspace_id, ip, title)
            .map(ApplicationResponse::SftpWatches),
        ApplicationCommand::ListTerminals => application
            .list_terminals()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::SelectTerminal {
            workspace_id,
            ip,
            title,
        } => application
            .select_terminal(workspace_id, ip, title)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ReadTerminal {
            workspace_id,
            offset,
            limit,
        } => application
            .read_terminal(workspace_id, offset, limit)
            .await
            .map(ApplicationResponse::TerminalRead),
        ApplicationCommand::SendText { workspace_id, text } => application
            .send_text(workspace_id, text)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::SendKey {
            workspace_id,
            key,
            control,
            alt,
            shift,
        } => application
            .send_key(workspace_id, key, control, alt, shift)
            .await
            .map(|_| ApplicationResponse::Empty),
    }
}

fn map_event(event: ApplicationEvent) -> Option<NotificationEnvelope> {
    let event = match event {
        ApplicationEvent::SessionOpened {
            workspace_id,
            profile,
        } => ApplicationNotification::SessionOpened {
            workspace_id,
            profile,
        },
        ApplicationEvent::SessionClosed { workspace_id } => {
            ApplicationNotification::SessionClosed { workspace_id }
        }
        ApplicationEvent::SessionSelected { workspace_id } => {
            ApplicationNotification::SessionSelected { workspace_id }
        }
    };
    Some(NotificationEnvelope { event })
}
