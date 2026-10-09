use std::time::Instant;

use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    application::{
        ApplicationResult,
        session::model::{ProfileSummary, SshTunnelWorkspaceSummary},
    },
    data_context::{SftpTransferSummary, SftpWatchSummary},
    domain::session::Protocol,
};

pub(crate) const COMMAND_CAPACITY: usize = 128;
pub(crate) const NOTIFICATION_CAPACITY: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RouteKey {
    Control,
    Workspace(String),
}

pub(crate) enum ApplicationCommand {
    ListProfiles,
    OpenSession {
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    },
    OpenSshTunnel {
        profile_id: String,
    },
    CloseSession {
        workspace_id: String,
    },
    ChangeSftpLocalDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    },
    ChangeSftpRemoteDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    },
    UploadSftp {
        workspace_id: String,
        local_paths: Vec<String>,
    },
    DownloadSftp {
        workspace_id: String,
        remote_paths: Vec<String>,
    },
    WatchSftpLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    },
    StopSftpLocalWatch {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    },
    SendText {
        workspace_id: String,
        text: String,
    },
    SendKey {
        workspace_id: String,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    },
}

impl ApplicationCommand {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::ListProfiles => "list_profiles",
            Self::OpenSession { .. } => "open_session",
            Self::OpenSshTunnel { .. } => "open_ssh_tunnel",
            Self::CloseSession { .. } => "close_session",
            Self::ChangeSftpLocalDirectory { .. } => "change_sftp_local_directory",
            Self::ChangeSftpRemoteDirectory { .. } => "change_sftp_remote_directory",
            Self::UploadSftp { .. } => "upload_sftp",
            Self::DownloadSftp { .. } => "download_sftp",
            Self::WatchSftpLocal { .. } => "watch_sftp_local",
            Self::StopSftpLocalWatch { .. } => "stop_sftp_local_watch",
            Self::SendText { .. } => "send_text",
            Self::SendKey { .. } => "send_key",
        }
    }

    pub(crate) fn workspace_id(&self) -> Option<&str> {
        match self {
            Self::CloseSession { workspace_id }
            | Self::ChangeSftpLocalDirectory { workspace_id, .. }
            | Self::ChangeSftpRemoteDirectory { workspace_id, .. }
            | Self::UploadSftp { workspace_id, .. }
            | Self::DownloadSftp { workspace_id, .. }
            | Self::WatchSftpLocal { workspace_id, .. }
            | Self::StopSftpLocalWatch { workspace_id, .. }
            | Self::SendText { workspace_id, .. }
            | Self::SendKey { workspace_id, .. } => Some(workspace_id),
            Self::ListProfiles | Self::OpenSession { .. } | Self::OpenSshTunnel { .. } => None,
        }
    }

    pub(crate) fn is_close_session(&self) -> bool {
        matches!(self, Self::CloseSession { .. })
    }

    pub(crate) fn route_key(&self) -> RouteKey {
        self.workspace_id()
            .map(|workspace_id| RouteKey::Workspace(workspace_id.to_owned()))
            .unwrap_or(RouteKey::Control)
    }
}

pub(crate) enum ApplicationResponse {
    Empty,
    Profiles(Vec<ProfileSummary>),
    WorkspaceId(String),
    SshTunnelWorkspace(SshTunnelWorkspaceSummary),
    SftpTransferSummary(SftpTransferSummary),
    SftpWatch(SftpWatchSummary),
}

#[derive(Clone, Debug)]
pub(crate) enum ApplicationNotification {
    ResyncRequired,
    SessionOpened {
        workspace_id: String,
        profile: ProfileSummary,
    },
    SessionClosed {
        workspace_id: String,
    },
    SessionSelected {
        workspace_id: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct NotificationEnvelope {
    pub(crate) event: ApplicationNotification,
}

pub(crate) struct CommandEnvelope {
    pub(crate) request_id: String,
    pub(crate) command: ApplicationCommand,
    pub(crate) response_tx: oneshot::Sender<ResponseEnvelope>,
    pub(crate) queued_at: Instant,
}

pub(crate) struct ResponseEnvelope {
    pub(crate) request_id: String,
    pub(crate) result: ApplicationResult<ApplicationResponse>,
}

#[derive(Clone)]
pub(crate) struct McpBridgeEndpoint {
    pub(crate) command_tx: mpsc::Sender<CommandEnvelope>,
    pub(crate) notification_tx: broadcast::Sender<NotificationEnvelope>,
    pub(crate) data_context: &'static crate::data_context::DataContext,
}

pub(crate) struct McpBridgeReceiver {
    pub(crate) command_rx: mpsc::Receiver<CommandEnvelope>,
    pub(crate) notification_tx: broadcast::Sender<NotificationEnvelope>,
    pub(crate) data_context: &'static crate::data_context::DataContext,
    pub(crate) notice: crate::data_context::DataContextNotice,
}
