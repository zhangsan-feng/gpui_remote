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
    model::SftpDirectorySummary, model::SftpTransferInfo, model::SftpTransferSummary,
    model::SftpWatchSummary, validation,
};

impl ApplicationContext {
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
        self.ssh().snapshot(workspace_id)
    }

    pub(crate) fn terminal_revision(&self, workspace_id: &str) -> ApplicationResult<u64> {
        self.ssh().revision(workspace_id)
    }

    pub(crate) fn sftp_snapshot(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<SftpDirectorySummary> {
        self.sftp().snapshot(workspace_id).map(map_remote_directory)
    }

    pub(crate) fn sftp_revision(&self, workspace_id: &str) -> ApplicationResult<u64> {
        self.sftp().revision(workspace_id)
    }

    pub(crate) fn sftp_connection_status(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<TerminalStatus> {
        self.sftp()
            .snapshot(workspace_id)
            .map(|snapshot| match snapshot.status {
                SftpStatus::Connecting => TerminalStatus::Connecting,
                SftpStatus::Connected => TerminalStatus::Connected,
                SftpStatus::Disconnected => TerminalStatus::Disconnected,
                SftpStatus::Failed => TerminalStatus::Failed,
            })
    }

    pub(crate) fn sftp_local_snapshot(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<SftpDirectorySummary> {
        self.sftp()
            .local_snapshot(workspace_id)
            .map(map_local_directory)
    }

    pub(crate) fn sftp_transfers_snapshot(
        &self,
        workspace_id: &str,
    ) -> ApplicationResult<Vec<SftpTransferInfo>> {
        self.sftp()
            .transfers(workspace_id)
            .map(|transfers| transfers.into_iter().map(map_transfer_info).collect())
    }

    pub(crate) async fn persist_sftp_local_path(
        &self,
        workspace_id: String,
        path: std::path::PathBuf,
    ) -> ApplicationResult<()> {
        self.update_sftp_local_path(&workspace_id, &path).await
    }

    pub(crate) async fn persist_sftp_remote_path(
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

    pub(crate) async fn delete_sftp_remote(
        &self,
        workspace_id: String,
        items: Vec<RemoteDeleteItem>,
    ) -> ApplicationResult<()> {
        self.sftp().delete_remote(&workspace_id, items).await
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
        self.sftp().load_directory(&workspace_id, path).await
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

    pub(crate) fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> ApplicationResult<Vec<SftpWatchSummary>> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .local_watches(&workspace_id)
            .map(|watches| watches.into_iter().map(map_watch_summary).collect())
    }

    pub(crate) async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<SftpWatchSummary> {
        validate_session(self, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.sftp()
            .watch_local(&workspace_id, local_path.into())
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
            .stop_watching_local(&workspace_id, local_path.as_ref())
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
    pub(crate) async fn list_profiles(
        &self,
    ) -> ApplicationResult<Vec<super::model::ProfileSummary>> {
        self.profile_query().await
    }

    pub(crate) async fn list_sftp_local(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<model::SftpDirectorySummary> {
        validation::validate_session_protocol(self, &workspace_id, Protocol::Sftp)?;
        self.sftp()
            .list_local(&workspace_id)
            .await
            .map(mapping::map_local_directory)
    }

    pub(crate) async fn list_sftp_sessions(
        &self,
    ) -> ApplicationResult<Vec<model::TerminalSummary>> {
        Ok(mapping::list_sftp_sessions(self))
    }

    pub(crate) async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<model::SftpDirectorySummary> {
        self.sftp()
            .snapshot(&workspace_id)
            .map(mapping::map_remote_directory)
    }

    pub(crate) async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<Vec<model::SftpTransferInfo>> {
        self.sftp().transfers(&workspace_id).map(|transfers| {
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
