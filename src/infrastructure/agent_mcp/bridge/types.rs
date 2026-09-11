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
