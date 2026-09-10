use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::domain::session::Protocol;

use super::{
    AgentMcpCommand, AgentSftpCommand, AgentSshCommand, SftpDirectorySummary, SftpTransferInfo,
    SftpTransferSummary, SftpWatchSummary, TerminalReadPage, TerminalSummary,
    command::AgentMcpResult,
};

const CHANNEL_CAPACITY: usize = 256;
const BRIDGE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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
    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> AgentMcpResult<String> {
        self.request(|reply| match protocol {
            Protocol::Ssh => AgentMcpCommand::Ssh(AgentSshCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }),
            Protocol::Sftp => AgentMcpCommand::Sftp(AgentSftpCommand::Open {
                profile_id,
                ip,
                title,
                reply,
            }),
        })
        .await
    }

    pub async fn list_sftp_local(&self) -> AgentMcpResult<SftpDirectorySummary> {
        self.request(|reply| AgentMcpCommand::Sftp(AgentSftpCommand::ListLocal { reply }))
            .await
    }

    pub async fn list_sftp_sessions(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.request(|reply| AgentMcpCommand::Sftp(AgentSftpCommand::ListSessions { reply }))
            .await
    }

    pub async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::ChangeLocalDirectory {
                workspace_id,
                ip,
                title,
                path,
                reply,
            })
        })
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

    pub async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::ChangeRemoteDirectory {
                workspace_id,
                ip,
                title,
                path,
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

    pub async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> AgentMcpResult<SftpWatchSummary> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::WatchLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            })
        })
        .await
    }

    pub async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::StopWatchingLocal {
                workspace_id,
                ip,
                title,
                local_path,
                reply,
            })
        })
        .await
    }

    pub async fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> AgentMcpResult<Vec<SftpWatchSummary>> {
        self.request(|reply| {
            AgentMcpCommand::Sftp(AgentSftpCommand::ListLocalWatches {
                workspace_id,
                ip,
                title,
                reply,
            })
        })
        .await
    }

    pub async fn list_terminals(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.request(|reply| AgentMcpCommand::Ssh(AgentSshCommand::ListTerminals { reply }))
            .await
    }

    pub async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> AgentMcpResult<()> {
        self.request(|reply| {
            AgentMcpCommand::Ssh(AgentSshCommand::SelectTerminal {
                workspace_id,
                ip,
                title,
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
        self.request_with_timeout(command, BRIDGE_REQUEST_TIMEOUT)
            .await
    }

    async fn request_with_timeout<T>(
        &self,
        command: impl FnOnce(oneshot::Sender<AgentMcpResult<T>>) -> AgentMcpCommand,
        timeout: Duration,
    ) -> AgentMcpResult<T> {
        let (reply, response) = oneshot::channel();
        let queue_capacity = self.commands.capacity();
        let deadline = tokio::time::Instant::now() + timeout;
        let command = command(reply);
        let command_name = command.name();

        if queue_capacity == 0 {
            log::warn!(
                "MCP GUI bridge command queue is full; command={command_name}, waiting for capacity"
            );
        }

        tokio::time::timeout_at(deadline, self.commands.send(command))
            .await
            .map_err(|_| {
                log::warn!(
                    "MCP GUI bridge command queue timed out after {:?}; command={command_name}, capacity_before_send={queue_capacity}",
                    timeout
                );
                "GUI MCP command queue timed out".to_owned()
            })?
            .map_err(|_| {
                log::warn!("MCP GUI bridge command rejected: command={command_name}");
                "GUI MCP bridge is unavailable".to_owned()
            })?;

        log::debug!(
            "MCP GUI bridge command dispatched: command={command_name}, capacity_after_send={}",
            self.commands.capacity()
        );

        let response = tokio::time::timeout_at(deadline, response)
            .await
            .map_err(|_| {
                log::warn!(
                    "MCP GUI bridge response timed out after {:?}: command={command_name}",
                    timeout
                );
                "GUI MCP request timed out".to_owned()
            })?
            .map_err(|_| "GUI MCP request was cancelled".to_owned())?;

        log::debug!("MCP GUI bridge response received: command={command_name}");
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_sftp_sessions_routes_to_the_sftp_command() {
        let (client, mut receiver) = agent_mcp_channel();
        let request = tokio::spawn(async move { client.list_sftp_sessions().await });

        let command = receiver
            .recv()
            .await
            .expect("SFTP session request expected");
        match command {
            AgentMcpCommand::Sftp(AgentSftpCommand::ListSessions { reply }) => {
                reply
                    .send(Ok(Vec::new()))
                    .expect("request should still be waiting");
            }
            _ => panic!("expected an SFTP session list command"),
        }

        assert!(request.await.expect("request task should finish").is_ok());
    }

    #[test]
    fn agent_mcp_channel_buffers_256_commands() {
        let (client, _receiver) = agent_mcp_channel();

        assert_eq!(client.commands.capacity(), 256);
    }

    #[tokio::test]
    async fn request_returns_an_error_when_the_command_queue_is_full() {
        let (client, _receiver) = agent_mcp_channel();

        for _ in 0..CHANNEL_CAPACITY {
            let (reply, _response) = oneshot::channel();
            client
                .commands
                .try_send(AgentMcpCommand::Ssh(AgentSshCommand::ListTerminals {
                    reply,
                }))
                .expect("the test queue should have capacity");
        }

        let result = client
            .request_with_timeout(
                |reply| AgentMcpCommand::Ssh(AgentSshCommand::ListTerminals { reply }),
                std::time::Duration::from_millis(20),
            )
            .await;

        assert!(matches!(result, Err(error) if error == "GUI MCP command queue timed out"));
    }
}
