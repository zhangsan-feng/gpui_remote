use tokio::sync::oneshot;

use super::{
    ProfileSummary, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary, TerminalReadPage,
    TerminalSummary,
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
        reply: oneshot::Sender<AgentMcpResult<String>>,
    },
    ListTerminals {
        reply: oneshot::Sender<AgentMcpResult<Vec<TerminalSummary>>>,
    },
    SelectTerminal {
        workspace_id: String,
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
        reply: oneshot::Sender<AgentMcpResult<String>>,
    },
    ListLocal {
        reply: oneshot::Sender<AgentMcpResult<SftpDirectorySummary>>,
    },
    ListRemote {
        workspace_id: String,
        reply: oneshot::Sender<AgentMcpResult<SftpDirectorySummary>>,
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
}
