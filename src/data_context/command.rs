use tokio::sync::oneshot;

use super::model::{
    SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary, SftpWatchSummary,
    TerminalReadPage, TerminalSummary,
};

pub type DataContextResult<T> = Result<T, String>;

pub enum DataContextCommand {
    Ssh(SshCommand),
    Sftp(SftpCommand),
}

pub enum SshCommand {
    Open {
        profile_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<DataContextResult<String>>,
    },
    ListTerminals {
        reply: oneshot::Sender<DataContextResult<Vec<TerminalSummary>>>,
    },
    SelectTerminal {
        workspace_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
    ReadTerminal {
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<DataContextResult<TerminalReadPage>>,
    },
    SendText {
        workspace_id: Option<String>,
        text: String,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
    SendKey {
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
}

pub enum SftpCommand {
    Open {
        profile_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<DataContextResult<String>>,
    },
    ListSessions {
        reply: oneshot::Sender<DataContextResult<Vec<TerminalSummary>>>,
    },
    ListLocal {
        reply: oneshot::Sender<DataContextResult<SftpDirectorySummary>>,
    },
    ChangeLocalDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
    ListRemote {
        workspace_id: String,
        reply: oneshot::Sender<DataContextResult<SftpDirectorySummary>>,
    },
    ChangeRemoteDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
    Upload {
        workspace_id: String,
        local_paths: Vec<String>,
        reply: oneshot::Sender<DataContextResult<SftpTransferSummary>>,
    },
    Download {
        workspace_id: String,
        remote_paths: Vec<String>,
        reply: oneshot::Sender<DataContextResult<SftpTransferSummary>>,
    },
    ListTransfers {
        workspace_id: String,
        reply: oneshot::Sender<DataContextResult<Vec<SftpTransferInfo>>>,
    },
    WatchLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        reply: oneshot::Sender<DataContextResult<SftpWatchSummary>>,
    },
    StopWatchingLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        reply: oneshot::Sender<DataContextResult<()>>,
    },
    ListLocalWatches {
        workspace_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<DataContextResult<Vec<SftpWatchSummary>>>,
    },
}

impl DataContextCommand {
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Ssh(command) => command.is_cancelled(),
            Self::Sftp(command) => command.is_cancelled(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Ssh(command) => command.name(),
            Self::Sftp(command) => command.name(),
        }
    }
}

impl SshCommand {
    fn is_cancelled(&self) -> bool {
        match self {
            Self::Open { reply, .. } => reply.is_closed(),
            Self::ListTerminals { reply } => reply.is_closed(),
            Self::SelectTerminal { reply, .. } => reply.is_closed(),
            Self::ReadTerminal { reply, .. } => reply.is_closed(),
            Self::SendText { reply, .. } => reply.is_closed(),
            Self::SendKey { reply, .. } => reply.is_closed(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Open { .. } => "ssh.open",
            Self::ListTerminals { .. } => "ssh.list_terminals",
            Self::SelectTerminal { .. } => "ssh.select_terminal",
            Self::ReadTerminal { .. } => "ssh.read_terminal",
            Self::SendText { .. } => "ssh.send_text",
            Self::SendKey { .. } => "ssh.send_key",
        }
    }
}

impl SftpCommand {
    fn is_cancelled(&self) -> bool {
        match self {
            Self::Open { reply, .. } => reply.is_closed(),
            Self::ListSessions { reply } => reply.is_closed(),
            Self::ListLocal { reply } => reply.is_closed(),
            Self::ChangeLocalDirectory { reply, .. } => reply.is_closed(),
            Self::ListRemote { reply, .. } => reply.is_closed(),
            Self::ChangeRemoteDirectory { reply, .. } => reply.is_closed(),
            Self::Upload { reply, .. } => reply.is_closed(),
            Self::Download { reply, .. } => reply.is_closed(),
            Self::ListTransfers { reply, .. } => reply.is_closed(),
            Self::WatchLocal { reply, .. } => reply.is_closed(),
            Self::StopWatchingLocal { reply, .. } => reply.is_closed(),
            Self::ListLocalWatches { reply, .. } => reply.is_closed(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Open { .. } => "sftp.open",
            Self::ListSessions { .. } => "sftp.list_sessions",
            Self::ListLocal { .. } => "sftp.list_local",
            Self::ChangeLocalDirectory { .. } => "sftp.change_local_directory",
            Self::ListRemote { .. } => "sftp.list_remote",
            Self::ChangeRemoteDirectory { .. } => "sftp.change_remote_directory",
            Self::Upload { .. } => "sftp.upload",
            Self::Download { .. } => "sftp.download",
            Self::ListTransfers { .. } => "sftp.list_transfers",
            Self::WatchLocal { .. } => "sftp.watch_local",
            Self::StopWatchingLocal { .. } => "sftp.stop_watching_local",
            Self::ListLocalWatches { .. } => "sftp.list_local_watches",
        }
    }
}
