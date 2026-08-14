use tokio::sync::{mpsc, oneshot};

use crate::domain::session::Protocol;

use super::{
    AgentMcpCommand, AgentSftpCommand, AgentSshCommand, ProfileSummary, SftpDirectorySummary,
    SftpTransferInfo, SftpTransferSummary, TerminalReadPage, TerminalSummary,
    command::AgentMcpResult,
};

const CHANNEL_CAPACITY: usize = 64;

#[derive(Clone)]
pub struct AgentMcpClient {
    commands: mpsc::Sender<AgentMcpCommand>,
}

pub struct AgentMcpReceiver {
    commands: mpsc::Receiver<AgentMcpCommand>,
}

pub fn agent_mcp_channel() -> (AgentMcpClient, AgentMcpReceiver) {
    let (commands, receiver) = mpsc::channel(CHANNEL_CAPACITY);
    (
        AgentMcpClient { commands },
        AgentMcpReceiver { commands: receiver },
    )
}

impl AgentMcpReceiver {
    pub async fn recv(&mut self) -> Option<AgentMcpCommand> {
        self.commands.recv().await
    }
}

impl AgentMcpClient {
    pub async fn list_profiles(&self) -> AgentMcpResult<Vec<ProfileSummary>> {
        self.request(|reply| AgentMcpCommand::ListProfiles { reply })
            .await
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
    ) -> AgentMcpResult<String> {
        self.request(|reply| match protocol {
            Protocol::Ssh => AgentMcpCommand::Ssh(AgentSshCommand::Open { profile_id, reply }),
            Protocol::Sftp => AgentMcpCommand::Sftp(AgentSftpCommand::Open { profile_id, reply }),
        })
        .await
    }

    pub async fn list_sftp_local(&self) -> AgentMcpResult<SftpDirectorySummary> {
        self.request(|reply| AgentMcpCommand::Sftp(AgentSftpCommand::ListLocal { reply }))
            .await
    }

    pub async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> AgentMcpResult<SftpDirectorySummary> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::ListRemote {
                workspace_id,
                reply,
            })
        })
        .await
    }

    pub async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> AgentMcpResult<SftpTransferSummary> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::Upload {
                workspace_id,
                local_paths,
                reply,
            })
        })
        .await
    }

    pub async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> AgentMcpResult<SftpTransferSummary> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::Download {
                workspace_id,
                remote_paths,
                reply,
            })
        })
        .await
    }

    pub async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> AgentMcpResult<Vec<SftpTransferInfo>> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::ListTransfers {
                workspace_id,
                reply,
            })
        })
        .await
    }

    pub async fn list_terminals(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.request(|reply| AgentMcpCommand::Ssh(AgentSshCommand::ListTerminals { reply }))
            .await
    }

    pub async fn select_terminal(&self, workspace_id: String) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Ssh(AgentSshCommand::SelectTerminal {
                workspace_id,
                reply,
            })
        })
        .await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> AgentMcpResult<TerminalReadPage> {
        self.request(|reply| {
            AgentMcpCommand::Ssh(AgentSshCommand::ReadTerminal {
                workspace_id,
                offset,
                limit,
                reply,
            })
        })
        .await
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Ssh(AgentSshCommand::SendText {
                workspace_id,
                text,
                reply,
            })
        })
        .await
    }

    pub async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Ssh(AgentSshCommand::SendKey {
                workspace_id,
                key,
                control,
                alt,
                shift,
                reply,
            })
        })
        .await
    }

    async fn request<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<AgentMcpResult<T>>) -> AgentMcpCommand,
    ) -> AgentMcpResult<T> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(command(reply))
            .await
            .map_err(|_| "GUI MCP bridge is unavailable".to_owned())?;
        response
            .await
            .map_err(|_| "GUI MCP request was cancelled".to_owned())?
    }
}
