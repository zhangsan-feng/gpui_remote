use tokio::sync::oneshot;

use super::{
    ProfileSummary, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary, SftpWatchSummary,
    TerminalReadPage, TerminalSummary,
};

pub type AgentMcpResult<T> = Result<T, String>;

pub enum AgentMcpCommand {
    ListProfiles {
        reply: oneshot::Sender<AgentMcpResult<Vec<ProfileSummary>>>,
    },
    Ssh(AgentSshCommand),
    Sftp(AgentSftpCommand),
}

pub enum AgentSshCommand {
    Open {
        profile_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<AgentMcpResult<String>>,
    },
    ListTerminals {
        reply: oneshot::Sender<AgentMcpResult<Vec<TerminalSummary>>>,
    },
    SelectTerminal {
        workspace_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
    ReadTerminal {
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<AgentMcpResult<TerminalReadPage>>,
    },
    SendText {
        workspace_id: Option<String>,
        text: String,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
    SendKey {
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
}

pub enum AgentSftpCommand {
    Open {
        profile_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<AgentMcpResult<String>>,
    },
    ListSessions {
        reply: oneshot::Sender<AgentMcpResult<Vec<TerminalSummary>>>,
    },
    ListLocal {
        reply: oneshot::Sender<AgentMcpResult<SftpDirectorySummary>>,
    },
    ChangeLocalDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
    ListRemote {
        workspace_id: String,
        reply: oneshot::Sender<AgentMcpResult<SftpDirectorySummary>>,
    },
    ChangeRemoteDirectory {
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
    Upload {
        workspace_id: String,
        local_paths: Vec<String>,
        reply: oneshot::Sender<AgentMcpResult<SftpTransferSummary>>,
    },
    Download {
        workspace_id: String,
        remote_paths: Vec<String>,
        reply: oneshot::Sender<AgentMcpResult<SftpTransferSummary>>,
    },
    ListTransfers {
        workspace_id: String,
        reply: oneshot::Sender<AgentMcpResult<Vec<SftpTransferInfo>>>,
    },
    WatchLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        reply: oneshot::Sender<AgentMcpResult<SftpWatchSummary>>,
    },
    StopWatchingLocal {
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
        reply: oneshot::Sender<AgentMcpResult<()>>,
    },
    ListLocalWatches {
        workspace_id: String,
        ip: String,
        title: String,
        reply: oneshot::Sender<AgentMcpResult<Vec<SftpWatchSummary>>>,
    },
}

impl AgentMcpCommand {
    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::ListProfiles { reply } => reply.is_closed(),
            Self::Ssh(command) => command.is_cancelled(),
            Self::Sftp(command) => command.is_cancelled(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::ListProfiles { .. } => "profiles.list",
            Self::Ssh(command) => command.name(),
            Self::Sftp(command) => command.name(),
        }
    }
}

impl AgentSshCommand {
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

impl AgentSftpCommand {
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
