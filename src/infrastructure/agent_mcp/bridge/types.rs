use std::time::Instant;

use tokio::sync::{broadcast, mpsc, oneshot};

use crate::{
    application::{
        ApplicationResult,
        model::{
            ProfileSummary, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary,
            SftpWatchSummary, TerminalReadPage, TerminalSummary,
        },
    },
    domain::session::Protocol,
};

pub(crate) const COMMAND_CAPACITY: usize = 128;
pub(crate) const NOTIFICATION_CAPACITY: usize = 256;

pub(crate) enum ApplicationCommand {
    ListProfiles,
    OpenSession {
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    },
    CloseSession {
        workspace_id: String,
    },
    ListSftpSessions,
    ListSftpLocal,
    ChangeSftpLocalDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    },
    ListSftpRemote {
        workspace_id: String,
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
    ListSftpTransfers {
        workspace_id: String,
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
    ListSftpLocalWatches {
        workspace_id: String,
        ip: String,
        title: String,
    },
    ListTerminals,
    SelectTerminal {
        workspace_id: String,
        ip: String,
        title: String,
    },
    ReadTerminal {
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    },
    SendText {
        workspace_id: Option<String>,
        text: String,
    },
    SendKey {
        workspace_id: Option<String>,
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
            Self::CloseSession { .. } => "close_session",
            Self::ListSftpSessions => "list_sftp_sessions",
            Self::ListSftpLocal => "list_sftp_local",
            Self::ChangeSftpLocalDirectory { .. } => "change_sftp_local_directory",
            Self::ListSftpRemote { .. } => "list_sftp_remote",
            Self::ChangeSftpRemoteDirectory { .. } => "change_sftp_remote_directory",
            Self::UploadSftp { .. } => "upload_sftp",
            Self::DownloadSftp { .. } => "download_sftp",
            Self::ListSftpTransfers { .. } => "list_sftp_transfers",
            Self::WatchSftpLocal { .. } => "watch_sftp_local",
            Self::StopSftpLocalWatch { .. } => "stop_sftp_local_watch",
            Self::ListSftpLocalWatches { .. } => "list_sftp_local_watches",
            Self::ListTerminals => "list_terminals",
            Self::SelectTerminal { .. } => "select_terminal",
            Self::ReadTerminal { .. } => "read_terminal",
            Self::SendText { .. } => "send_text",
            Self::SendKey { .. } => "send_key",
        }
    }

    pub(crate) fn workspace_id(&self) -> Option<&str> {
        match self {
            Self::CloseSession { workspace_id }
            | Self::ListSftpRemote { workspace_id }
            | Self::UploadSftp { workspace_id, .. }
            | Self::DownloadSftp { workspace_id, .. }
            | Self::ListSftpTransfers { workspace_id }
            | Self::WatchSftpLocal { workspace_id, .. }
            | Self::StopSftpLocalWatch { workspace_id, .. }
            | Self::ListSftpLocalWatches { workspace_id, .. }
            | Self::SelectTerminal { workspace_id, .. } => Some(workspace_id),
            Self::ReadTerminal { workspace_id, .. }
            | Self::SendText { workspace_id, .. }
            | Self::SendKey { workspace_id, .. } => workspace_id.as_deref(),
            Self::ListProfiles
            | Self::OpenSession { .. }
            | Self::ListSftpSessions
            | Self::ListSftpLocal
            | Self::ChangeSftpLocalDirectory { .. }
            | Self::ChangeSftpRemoteDirectory { .. }
            | Self::ListTerminals => None,
        }
    }
}

pub(crate) enum ApplicationResponse {
    Empty,
    Profiles(Vec<ProfileSummary>),
    WorkspaceId(String),
    TerminalSummaries(Vec<TerminalSummary>),
    TerminalRead(TerminalReadPage),
    SftpDirectory(SftpDirectorySummary),
    SftpTransferSummary(SftpTransferSummary),
    SftpTransferInfos(Vec<SftpTransferInfo>),
    SftpWatch(SftpWatchSummary),
    SftpWatches(Vec<SftpWatchSummary>),
}

#[derive(Clone, Debug)]
pub(crate) enum ApplicationNotification {
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
}

pub(crate) struct McpBridgeReceiver {
    pub(crate) command_rx: mpsc::Receiver<CommandEnvelope>,
    pub(crate) notification_tx: broadcast::Sender<NotificationEnvelope>,
}
