use crate::domain::session::Protocol;

use super::{
    DataContextResult, GuiContext, ProfileSummary, QueryService, SftpDirectorySummary,
    SftpTransferInfo, SftpTransferSummary, SftpWatchSummary, TerminalReadPage, TerminalSummary,
};

#[derive(Clone)]
pub struct McpContext {
    gui: GuiContext,
    queries: QueryService,
}

impl McpContext {
    pub(crate) fn new(gui: GuiContext, queries: QueryService) -> Self {
        Self { gui, queries }
    }

    pub async fn list_profiles(&self) -> DataContextResult<Vec<ProfileSummary>> {
        self.queries.list_profiles().await
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> DataContextResult<String> {
        self.gui.open_session(profile_id, protocol, ip, title).await
    }

    pub async fn list_sftp_local(&self) -> DataContextResult<SftpDirectorySummary> {
        self.gui.list_sftp_local().await
    }

    pub async fn list_sftp_sessions(&self) -> DataContextResult<Vec<TerminalSummary>> {
        self.gui.list_sftp_sessions().await
    }

    pub async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        self.gui
            .change_sftp_local_directory(workspace_id, ip, title, path)
            .await
    }

    pub async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> DataContextResult<SftpDirectorySummary> {
        self.gui.list_sftp_remote(workspace_id).await
    }

    pub async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        self.gui
            .change_sftp_remote_directory(workspace_id, ip, title, path)
            .await
    }

    pub async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.gui.upload_sftp(workspace_id, local_paths).await
    }

    pub async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.gui.download_sftp(workspace_id, remote_paths).await
    }

    pub async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> DataContextResult<Vec<SftpTransferInfo>> {
        self.gui.list_sftp_transfers(workspace_id).await
    }

    pub async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<SftpWatchSummary> {
        self.gui
            .watch_sftp_local(workspace_id, ip, title, local_path)
            .await
    }

    pub async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<()> {
        self.gui
            .stop_sftp_local_watch(workspace_id, ip, title, local_path)
            .await
    }

    pub async fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<Vec<SftpWatchSummary>> {
        self.gui.list_sftp_local_watches(workspace_id, ip, title)
    }

    pub async fn list_terminals(&self) -> DataContextResult<Vec<TerminalSummary>> {
        self.gui.list_terminals().await
    }

    pub async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<()> {
        self.gui.select_terminal(workspace_id, ip, title).await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> DataContextResult<TerminalReadPage> {
        self.gui.read_terminal(workspace_id, offset, limit).await
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> DataContextResult<()> {
        self.gui.send_text(workspace_id, text).await
    }

    pub async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> DataContextResult<()> {
        self.gui
            .send_key(workspace_id, key, control, alt, shift)
            .await
    }
}
