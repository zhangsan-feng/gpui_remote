use crate::{
    application::{
        ApplicationResult,
        session::validation,
        sftp::{RemoteDeleteItem, mapping},
    },
    data_context::{SftpTransferSummary, SftpWatchSummary},
    domain::session::Protocol,
};

use super::SftpApplication;

impl SftpApplication {
    pub(crate) async fn delete_local_paths_checked(
        &self,
        workspace_id: String,
        paths: Vec<std::path::PathBuf>,
    ) -> ApplicationResult<()> {
        self.delete_local_paths(&workspace_id, paths)
            .await
            .map(|_| ())
    }

    pub(crate) async fn delete_remote_paths_checked(
        &self,
        workspace_id: String,
        items: Vec<RemoteDeleteItem>,
    ) -> ApplicationResult<()> {
        self.delete_remote_paths(&workspace_id, items).await
    }

    pub(crate) async fn change_local_directory_checked(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        validation::validate_session(&self.sessions, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.change_local_directory(&workspace_id, path.into())
            .await
            .map(|_| ())
    }

    pub(crate) async fn change_remote_directory_checked(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        validation::validate_session(&self.sessions, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.change_remote_directory(&workspace_id, path)
            .await
            .map(|_| ())
    }

    pub(crate) async fn upload_checked(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        self.upload(&workspace_id, local_paths)
            .await
            .map(mapping::map_transfer_summary)
    }

    pub(crate) async fn download_checked(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        self.download(&workspace_id, remote_paths)
            .await
            .map(mapping::map_transfer_summary)
    }

    pub(crate) async fn cancel_transfer_checked(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> ApplicationResult<()> {
        self.cancel_transfer(&workspace_id, transfer_id)
    }

    pub(crate) async fn retry_transfer_checked(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> ApplicationResult<()> {
        self.retry_transfer(&workspace_id, transfer_id).await
    }

    pub(crate) async fn start_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<SftpWatchSummary> {
        validation::validate_session(&self.sessions, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.listen_local_directory(&workspace_id, local_path.into())
            .await
    }

    pub(crate) async fn stop_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<()> {
        validation::validate_session(&self.sessions, &workspace_id, Protocol::Sftp, &ip, &title)?;
        self.stop_listening_local_directory(&workspace_id, local_path.as_ref())
            .await
    }
}
