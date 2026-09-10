use crate::domain::session::Protocol;

use super::{
    AgentMcpClient, AgentMcpQueryService, ProfileSummary, SftpDirectorySummary, SftpTransferInfo,
    SftpTransferSummary, SftpWatchSummary, TerminalReadPage, TerminalSummary,
    command::AgentMcpResult,
};

#[derive(Clone)]
pub struct AgentMcpDataFlow {
    commands: AgentMcpClient,
    queries: AgentMcpQueryService,
}

impl AgentMcpDataFlow {
    pub fn new(commands: AgentMcpClient, queries: AgentMcpQueryService) -> Self {
        Self { commands, queries }
    }

    pub async fn list_profiles(&self) -> AgentMcpResult<Vec<ProfileSummary>> {
        self.queries.list_profiles().await
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> AgentMcpResult<String> {
        self.commands
            .open_session(profile_id, protocol, ip, title)
            .await
    }

    pub async fn list_sftp_local(&self) -> AgentMcpResult<SftpDirectorySummary> {
        self.commands.list_sftp_local().await
    }

    pub async fn list_sftp_sessions(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.commands.list_sftp_sessions().await
    }

    pub async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> AgentMcpResult<()> {
        self.commands
            .change_sftp_local_directory(workspace_id, ip, title, path)
            .await
    }

    pub async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> AgentMcpResult<SftpDirectorySummary> {
        self.commands.list_sftp_remote(workspace_id).await
    }

    pub async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> AgentMcpResult<()> {
        self.commands
            .change_sftp_remote_directory(workspace_id, ip, title, path)
            .await
    }

    pub async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> AgentMcpResult<SftpTransferSummary> {
        self.commands.upload_sftp(workspace_id, local_paths).await
    }

    pub async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> AgentMcpResult<SftpTransferSummary> {
        self.commands
            .download_sftp(workspace_id, remote_paths)
            .await
    }

    pub async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> AgentMcpResult<Vec<SftpTransferInfo>> {
        self.commands.list_sftp_transfers(workspace_id).await
    }

    pub async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> AgentMcpResult<SftpWatchSummary> {
        self.commands
            .watch_sftp_local(workspace_id, ip, title, local_path)
            .await
    }

    pub async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> AgentMcpResult<()> {
        self.commands
            .stop_sftp_local_watch(workspace_id, ip, title, local_path)
            .await
    }

    pub async fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> AgentMcpResult<Vec<SftpWatchSummary>> {
        self.commands
            .list_sftp_local_watches(workspace_id, ip, title)
            .await
    }

    pub async fn list_terminals(&self) -> AgentMcpResult<Vec<TerminalSummary>> {
        self.commands.list_terminals().await
    }

    pub async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> AgentMcpResult<()> {
        self.commands.select_terminal(workspace_id, ip, title).await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> AgentMcpResult<TerminalReadPage> {
        self.commands
            .read_terminal(workspace_id, offset, limit)
            .await
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> AgentMcpResult<()> {
        self.commands.send_text(workspace_id, text).await
    }

    pub async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> AgentMcpResult<()> {
        self.commands
            .send_key(workspace_id, key, control, alt, shift)
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::super::{
        AgentMcpDataFlow, AgentMcpProfileQuery, AgentMcpQueryService, ProfileSummary,
        agent_mcp_channel,
    };

    struct StaticProfileQuery;

    impl AgentMcpProfileQuery for StaticProfileQuery {
        fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
            Ok(vec![ProfileSummary {
                id: "profile-1".to_owned(),
                title: "Local".to_owned(),
                host: "127.0.0.1".to_owned(),
                protocol: "ssh".to_owned(),
            }])
        }
    }

    #[tokio::test]
    async fn list_profiles_bypasses_the_gui_command_channel() {
        let (commands, mut receiver) = agent_mcp_channel();
        let queries = AgentMcpQueryService::new(Arc::new(StaticProfileQuery));
        let data_flow = AgentMcpDataFlow::new(commands, queries);

        let profiles = data_flow
            .list_profiles()
            .await
            .expect("profile query should succeed");

        assert_eq!(profiles[0].id, "profile-1");
        assert!(
            tokio::time::timeout(Duration::from_millis(20), receiver.recv())
                .await
                .is_err()
        );
    }
}
