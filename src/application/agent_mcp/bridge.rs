use tokio::sync::{mpsc, oneshot};

use super::{
    AgentMcpCommand, ProfileSummary, TerminalReadPage, TerminalSummary, command::AgentMcpResult,
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

    pub async fn open_session(&self, profile_id: String) -> AgentMcpResult<String> {
        self.request(|reply| AgentMcpCommand::OpenSession { profile_id, reply })
            .await
    }

    pub async fn list_terminals(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.request(|reply| AgentMcpCommand::ListTerminals { reply })
            .await
    }

    pub async fn select_terminal(&self, workspace_id: String) -> AgentMcpResult<()> {
        self.request(|reply| AgentMcpCommand::SelectTerminal {
            workspace_id,
            reply,
        })
        .await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> AgentMcpResult<TerminalReadPage> {
        self.request(|reply| AgentMcpCommand::ReadTerminal {
            workspace_id,
            offset,
            limit,
            reply,
        })
        .await
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| AgentMcpCommand::SendText {
            workspace_id,
            text,
            reply,
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
        self.request(|reply| AgentMcpCommand::SendKey {
            workspace_id,
            key,
            control,
            alt,
            shift,
            reply,
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
