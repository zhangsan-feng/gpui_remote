use crate::application::{
    ApplicationContext, ApplicationEvent, ApplicationResult, model::ProfileSummary,
};

use super::types::{
    ApplicationCommand, ApplicationNotification, ApplicationResponse, NotificationEnvelope,
};

pub(crate) async fn dispatch(
    application: &ApplicationContext,
    command: ApplicationCommand,
) -> ApplicationResult<ApplicationResponse> {
    match command {
        ApplicationCommand::ListProfiles => application
            .list_profile_summaries()
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
        ApplicationCommand::CloseSession { workspace_id } => application
            .close_session(&workspace_id)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ListSftpSessions => application
            .list_sftp_workspace_summaries()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::ListSftpLocal { workspace_id } => application
            .read_sftp_local_directory(workspace_id)
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
            .read_sftp_remote_directory(workspace_id)
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
            .read_sftp_transfer_records(workspace_id)
            .await
            .map(ApplicationResponse::SftpTransferInfos),
        ApplicationCommand::WatchSftpLocal {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .start_sftp_local_watch(workspace_id, ip, title, local_path)
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
            .sftp_local_watch_summaries(workspace_id, ip, title)
            .map(ApplicationResponse::SftpWatches),
        ApplicationCommand::ListTerminals => application
            .list_terminals()
            .await
            .map(ApplicationResponse::TerminalSummaries),
        ApplicationCommand::McpReadTerminal {
            workspace_id,
            offset,
            limit,
            since_mcp_snapshot_version,
        } => application
            .mcp_read_terminal(workspace_id, offset, limit, since_mcp_snapshot_version)
            .await
            .map(ApplicationResponse::McpTerminalRead),
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

pub(crate) fn map_event(event: ApplicationEvent) -> NotificationEnvelope {
    let event = match event {
        ApplicationEvent::SessionOpened {
            workspace_id,
            profile,
        } => ApplicationNotification::SessionOpened {
            workspace_id,
            profile: ProfileSummary {
                id: profile.id,
                title: profile.name,
                host: profile.host,
                protocol: profile.connection_protocol.as_str().to_owned(),
            },
        },
        ApplicationEvent::SessionClosed { workspace_id } => {
            ApplicationNotification::SessionClosed { workspace_id }
        }
        ApplicationEvent::SessionSelected { workspace_id } => {
            ApplicationNotification::SessionSelected { workspace_id }
        }
    };
    NotificationEnvelope { event }
}
