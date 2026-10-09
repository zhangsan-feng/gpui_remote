use crate::{
    application::sftp_server::{
        SftpServerAttributes, SftpServerDirectoryEntry, SftpServerMetadata, SftpServerOpenOptions,
        SftpServerRawHandle,
    },
    infrastructure::InfrastructureContext,
};

impl InfrastructureContext {
    pub(crate) async fn sftp_server_canonicalize(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<std::path::PathBuf> {
        self.inner.sftp_server_filesystem.canonicalize(path).await
    }

    pub(crate) async fn sftp_server_open(
        &self,
        path: std::path::PathBuf,
        options: SftpServerOpenOptions,
    ) -> std::io::Result<SftpServerRawHandle> {
        self.inner.sftp_server_filesystem.open(path, options).await
    }

    pub(crate) async fn sftp_server_open_directory(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<SftpServerRawHandle> {
        self.inner.sftp_server_filesystem.open_directory(path).await
    }

    pub(crate) async fn sftp_server_close(
        &self,
        handle: SftpServerRawHandle,
    ) -> std::io::Result<()> {
        self.inner.sftp_server_filesystem.close(handle).await
    }

    pub(crate) async fn sftp_server_read(
        &self,
        handle: SftpServerRawHandle,
        offset: u64,
        len: u32,
    ) -> std::io::Result<Vec<u8>> {
        self.inner
            .sftp_server_filesystem
            .read(handle, offset, len)
            .await
    }

    pub(crate) async fn sftp_server_write(
        &self,
        handle: SftpServerRawHandle,
        offset: u64,
        data: Vec<u8>,
    ) -> std::io::Result<()> {
        self.inner
            .sftp_server_filesystem
            .write(handle, offset, data)
            .await
    }

    pub(crate) async fn sftp_server_metadata(
        &self,
        path: std::path::PathBuf,
        follow_links: bool,
    ) -> std::io::Result<SftpServerMetadata> {
        self.inner
            .sftp_server_filesystem
            .metadata(path, follow_links)
            .await
    }

    pub(crate) async fn sftp_server_metadata_handle(
        &self,
        handle: SftpServerRawHandle,
    ) -> std::io::Result<SftpServerMetadata> {
        self.inner
            .sftp_server_filesystem
            .metadata_handle(handle)
            .await
    }

    pub(crate) async fn sftp_server_read_directory(
        &self,
        handle: SftpServerRawHandle,
    ) -> std::io::Result<Option<SftpServerDirectoryEntry>> {
        self.inner
            .sftp_server_filesystem
            .read_directory(handle)
            .await
    }

    pub(crate) async fn sftp_server_set_attributes(
        &self,
        path: std::path::PathBuf,
        attributes: SftpServerAttributes,
    ) -> std::io::Result<()> {
        self.inner
            .sftp_server_filesystem
            .set_attributes(path, attributes)
            .await
    }

    pub(crate) async fn sftp_server_create_directory(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<()> {
        self.inner
            .sftp_server_filesystem
            .create_directory(path)
            .await
    }

    pub(crate) async fn sftp_server_remove_file(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<()> {
        self.inner.sftp_server_filesystem.remove_file(path).await
    }

    pub(crate) async fn sftp_server_remove_directory(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<()> {
        self.inner
            .sftp_server_filesystem
            .remove_directory(path)
            .await
    }

    pub(crate) async fn sftp_server_rename(
        &self,
        old_path: std::path::PathBuf,
        new_path: std::path::PathBuf,
    ) -> std::io::Result<()> {
        self.inner
            .sftp_server_filesystem
            .rename(old_path, new_path)
            .await
    }

    pub(crate) async fn sftp_server_read_link(
        &self,
        path: std::path::PathBuf,
    ) -> std::io::Result<std::path::PathBuf> {
        self.inner.sftp_server_filesystem.read_link(path).await
    }
}
