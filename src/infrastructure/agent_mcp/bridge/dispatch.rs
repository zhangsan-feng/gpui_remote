use crate::{
    application::{Application, ApplicationResult, session::model::ProfileSummary},
    data_context::DataChange,
};

use super::types::{
    ApplicationCommand, ApplicationNotification, ApplicationResponse, NotificationEnvelope,
};

pub(crate) async fn dispatch(
    application: &Application,
    command: ApplicationCommand,
) -> ApplicationResult<ApplicationResponse> {
    let _accepted = application.begin_operation().await?;
    match command {
        ApplicationCommand::ListProfiles => application
            .sessions
            .list_profile_summaries()
            .await
            .map(ApplicationResponse::Profiles),
        ApplicationCommand::OpenSession {
            profile_id,
            protocol,
            ip,
            title,
        } => application
            .open_session(profile_id, protocol, ip, title, false)
            .await
            .map(ApplicationResponse::WorkspaceId),
        ApplicationCommand::OpenSshTunnel { profile_id } => application
            .open_ssh_tunnel(profile_id)
            .await
            .map(ApplicationResponse::SshTunnelWorkspace),
        ApplicationCommand::CloseSession { workspace_id } => application
            .close_session(&workspace_id)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ChangeSftpLocalDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .sftp
            .change_local_directory_checked(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::ChangeSftpRemoteDirectory {
            workspace_id,
            ip,
            title,
            path,
        } => application
            .sftp
            .change_remote_directory_checked(workspace_id, ip, title, path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::UploadSftp {
            workspace_id,
            local_paths,
        } => application
            .sftp
            .upload_checked(workspace_id, local_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::DownloadSftp {
            workspace_id,
            remote_paths,
        } => application
            .sftp
            .download_checked(workspace_id, remote_paths)
            .await
            .map(ApplicationResponse::SftpTransferSummary),
        ApplicationCommand::WatchSftpLocal {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .sftp
            .start_local_watch(workspace_id, ip, title, local_path)
            .await
            .map(ApplicationResponse::SftpWatch),
        ApplicationCommand::StopSftpLocalWatch {
            workspace_id,
            ip,
            title,
            local_path,
        } => application
            .sftp
            .stop_local_watch(workspace_id, ip, title, local_path)
            .await
            .map(|_| ApplicationResponse::Empty),
        ApplicationCommand::SendText { workspace_id, text } => application
            .ssh
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
            .ssh
            .send_key_checked(workspace_id, key, control, alt, shift)
            .await
            .map(|_| ApplicationResponse::Empty),
    }
}

pub(crate) fn map_event(event: DataChange) -> NotificationEnvelope {
    let event = match event {
        DataChange::SessionOpened {
            workspace_id,
            profile,
        } => ApplicationNotification::SessionOpened {
            workspace_id,
            profile: ProfileSummary {
                id: profile.profile_id,
                title: profile.title,
                host: profile.host,
                protocol: profile.connection_protocol.as_str().to_owned(),
            },
        },
        DataChange::SessionClosed { workspace_id } => {
            ApplicationNotification::SessionClosed { workspace_id }
        }
        DataChange::SessionSelected { workspace_id } => {
            ApplicationNotification::SessionSelected { workspace_id }
        }
    };
    NotificationEnvelope { event }
}
