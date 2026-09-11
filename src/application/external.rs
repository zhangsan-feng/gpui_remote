use std::sync::Arc;

use crate::{
    application::mapping::{
        map_local_directory, map_remote_directory, map_transfer_info, map_transfer_summary,
        map_watch_summary,
    },
    application::validation::validate_session,
    domain::{session::Protocol, terminal::TerminalStatus},
};

use super::{
    ApplicationContext, ApplicationResult, RemoteDeleteItem, SftpStatus, mapping, model,
    model::SftpTransferSummary, model::SftpWatchSummary, validation,
};

impl ApplicationContext {
    pub(crate) fn workspace_runtime_available(&self, workspace_id: &str) -> bool {
        let Some(profile) = self.sessions().profile_for_workspace(workspace_id) else {
            return false;
        };
        match profile.protocol {
            Protocol::Ssh => self.ssh().runtime_available(workspace_id),
            Protocol::Sftp => self.sftp().runtime_available(workspace_id),
        }
    }

    pub(crate) fn terminal_updates(&self) -> Arc<tokio::sync::Notify> {
        self.ssh().updates()
    }

    pub(crate) fn terminal_status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.ssh().status_updates()
    }

    pub(crate) fn sftp_updates(&self) -> Arc<tokio::sync::Notify> {
        self.sftp().updates()
    }

    pub(crate) fn sftp_status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.sftp().status_updates()
    }

    pub(crate) fn terminal_snapshot(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<crate::domain::terminal::TerminalData> {
        self.ssh().terminal_snapshot(workspace_id)
    }

    pub(crate) fn terminal_revision(&self, workspace_id: &str) -> ApplicationResult<u64> {
        self.ssh().terminal_revision(workspace_id)
    }

    pub(crate) fn sftp_workspace_snapshot(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<model::SftpWorkspaceSnapshot> {
        validation::validate_session_protocol(self, workspace_id, Protocol::Sftp)?;
        let remote_snapshot = self.sftp().sftp_remote_snapshot(workspace_id)?;
        let status = match remote_snapshot.status {
            SftpStatus::Connecting => "connecting",
            SftpStatus::Connected => "connected",
            SftpStatus::Disconnected => "disconnected",
            SftpStatus::Failed => "failed",
        };
        let remote = map_remote_directory(remote_snapshot);
        let local = self
            .sftp()
            .local_directory_snapshot(workspace_id)
            .map(map_local_directory)?;
        let transfers = self
            .sftp()
            .sftp_transfer_snapshot(workspace_id)
            .map(|transfers| transfers.into_iter().map(map_transfer_info).collect())?;
        let watches = self
            .sftp()
            .sftp_local_watch_summaries(workspace_id)
            .map(|watches| watches.into_iter().map(map_watch_summary).collect())?;
        let remote_revision = self.sftp().sftp_remote_revision(workspace_id)?;
        Ok(model::SftpWorkspaceSnapshot {
            remote,
            local,
            transfers,
            watches,
            status: status.to_owned(),
            remote_revision,
        })
    }

    pub(crate) fn sftp_connection_status(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<TerminalStatus> {
        self.sftp()
            .sftp_remote_snapshot(workspace_id)
            .map(|snapshot| match snapshot.status {
                SftpStatus::Connecting => TerminalStatus::Connecting,
                SftpStatus::Connected => TerminalStatus::Connected,
                SftpStatus::Disconnected => TerminalStatus::Disconnected,
                SftpStatus::Failed => TerminalStatus::Failed,
            })
    }

    pub(crate) async fn save_sftp_local_path(
        &self,
        workspace_id: String,
        path: std::path::PathBuf,
    ) -> ApplicationResult<()> {
        self.update_sftp_local_path(&workspace_id, &path).await
    }

    pub(crate) async fn save_sftp_remote_path(
        &self,
        workspace_id: String,
        path: String,
    ) -> ApplicationResult<()> {
        self.update_sftp_remote_path(&workspace_id, path).await
    }

    pub(crate) async fn delete_sftp_local_paths(
        &self,
        workspace_id: String,
        paths: Vec<std::path::PathBuf>,
    ) -> ApplicationResult<()> {
        self.sftp()
            .delete_local_paths(&workspace_id, paths)
            .await
            .map(|_| ())
    }

    pub(crate) async fn delete_sftp_remote_paths(
        &self,
        workspace_id: String,
        items: Vec<RemoteDeleteItem>,
    ) -> ApplicationResult<()> {
        self.sftp().delete_remote_paths(&workspace_id, items).await
    }

    pub(crate) async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .change_local_directory(&workspace_id, path.into())
            .await
            .map(|_| ())
    }

    pub(crate) async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .change_remote_directory(&workspace_id, path)
            .await
    }

    pub(crate) async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        self.sftp()
            .upload(&workspace_id, local_paths)
            .await
            .map(map_transfer_summary)
    }

    pub(crate) async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        self.sftp()
            .download(&workspace_id, remote_paths)
            .await
            .map(map_transfer_summary)
    }

    pub(crate) async fn cancel_sftp_transfer(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> ApplicationResult<()> {
        self.sftp().cancel_transfer(&workspace_id, transfer_id)
    }

    pub(crate) async fn retry_sftp_transfer(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> ApplicationResult<()> {
        self.sftp().retry_transfer(&workspace_id, transfer_id).await
    }

    pub(crate) fn sftp_local_watch_summaries(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> ApplicationResult<Vec<SftpWatchSummary>> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .sftp_local_watch_summaries(&workspace_id)
            .map(|watches| watches.into_iter().map(map_watch_summary).collect())
    }

    pub(crate) async fn start_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<SftpWatchSummary> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .listen_local_directory(&workspace_id, local_path.into())
            .await
            .map(map_watch_summary)
    }

    pub(crate) async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<()> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .stop_listening_local_directory(&workspace_id, local_path.as_ref())
            .await
    }

    pub(crate) async fn send_terminal_input(
        &self,
        workspace_id: String,
        input: Vec<u8>,
    ) -> ApplicationResult<()> {
        self.ssh().send_input(&workspace_id, input).await
    }

    pub(crate) async fn resize_terminal(
        &self,
        workspace_id: String,
        columns: u32,
        rows: u32,
    ) -> ApplicationResult<()> {
        self.ssh().resize(&workspace_id, columns, rows).await
    }

    pub(crate) async fn scroll_terminal(
        &self,
        workspace_id: String,
        lines: i32,
    ) -> ApplicationResult<()> {
        self.ssh().scroll(&workspace_id, lines).await
    }

    pub(crate) async fn scroll_terminal_to(
        &self,
        workspace_id: String,
        offset: usize,
    ) -> ApplicationResult<()> {
        self.ssh().scroll_to(&workspace_id, offset).await
    }
}

impl ApplicationContext {
    pub(crate) async fn list_profile_summaries(
        &self,
    ) -> ApplicationResult<Vec<super::model::ProfileSummary>> {
        self.list_session_profiles().await.map(|profiles| {
            profiles
                .into_iter()
                .map(mapping::map_profile_summary)
                .collect()
        })
    }

    pub(crate) async fn read_sftp_local_directory(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<model::SftpDirectorySummary> {
        validation::validate_session_protocol(self, &workspace_id, Protocol::Sftp)?;
        self.sftp()
            .local_directory_snapshot(&workspace_id)
            .map(mapping::map_local_directory)
    }

    pub(crate) async fn list_sftp_workspace_summaries(
        &self,
    ) -> ApplicationResult<Vec<model::TerminalSummary>> {
        Ok(mapping::list_sftp_sessions(self))
    }

    pub(crate) async fn read_sftp_remote_directory(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<model::SftpDirectorySummary> {
        self.sftp()
            .sftp_remote_snapshot(&workspace_id)
            .map(mapping::map_remote_directory)
    }

    pub(crate) async fn read_sftp_transfer_records(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<Vec<model::SftpTransferInfo>> {
        self.sftp()
            .sftp_transfer_snapshot(&workspace_id)
            .map(|transfers| {
                transfers
                    .into_iter()
                    .map(mapping::map_transfer_info)
                    .collect()
            })
    }

    pub(crate) async fn list_terminals(&self) -> ApplicationResult<Vec<model::TerminalSummary>> {
        Ok(mapping::list_terminals(self))
    }

    pub(crate) async fn read_terminal(
        &self,
        workspace_id: String,
        offset: usize,
        limit: usize,
    ) -> ApplicationResult<model::TerminalReadPage> {
        validation::validate_terminal_workspace(self, &workspace_id)?;
        self.ssh()
            .read(&workspace_id, offset, mapping::normalize_read_limit(limit))
            .await
            .map(|page| model::TerminalReadPage {
                workspace_id,
                text: page.text,
                total_lines: page.total_lines,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
            })
    }

    pub(crate) async fn send_text(
        &self,
        workspace_id: String,
        text: String,
    ) -> ApplicationResult<()> {
        validation::validate_terminal_workspace(self, &workspace_id)?;
        self.ssh()
            .send_input(&workspace_id, text.into_bytes())
            .await
    }

    pub(crate) async fn send_key(
        &self,
        workspace_id: String,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> ApplicationResult<()> {
        validation::validate_terminal_workspace(self, &workspace_id)?;
        self.ssh()
            .send_key(&workspace_id, &key, control, alt, shift)
            .await
    }
}
