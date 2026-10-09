use std::{collections::HashMap, path::PathBuf, sync::Arc};

use russh_sftp::{
    protocol::{
        Attrs, Data, File as SftpFile, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
        Version,
    },
    server::Handler,
};

use crate::application::sftp_server::{
    SftpServerApplication, SftpServerAttributes, SftpServerError, SftpServerFileType,
    SftpServerMetadata, SftpServerOpenOptions,
};

pub(crate) struct LocalSftpHandler {
    application: Arc<SftpServerApplication>,
    session_id: u64,
    version: Option<u32>,
}

impl LocalSftpHandler {
    pub(crate) fn new(application: Arc<SftpServerApplication>, root: PathBuf) -> Self {
        let session_id = application.open_session(root);
        Self {
            application,
            session_id,
            version: None,
        }
    }

    fn status(id: u32, status_code: StatusCode) -> Status {
        Status {
            id,
            status_code,
            error_message: status_code.to_string(),
            language_tag: "en-US".to_owned(),
        }
    }
}

impl Drop for LocalSftpHandler {
    fn drop(&mut self) {
        let application = self.application.clone();
        let session_id = self.session_id;
        tokio::spawn(async move {
            if let Err(error) = application.close_session(session_id).await {
                log::debug!("sftp_server_session_cleanup_failed error={error:?}");
            }
        });
    }
}

impl Handler for LocalSftpHandler {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, Self::Error> {
        if self.version.replace(version).is_some() {
            return Err(StatusCode::ConnectionLost);
        }
        Ok(Version::new())
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let write = pflags.intersects(OpenFlags::WRITE | OpenFlags::APPEND | OpenFlags::TRUNCATE);
        let handle = self
            .application
            .open(
                self.session_id,
                filename,
                SftpServerOpenOptions {
                    read: pflags.contains(OpenFlags::READ) || !write,
                    write,
                    append: pflags.contains(OpenFlags::APPEND),
                    truncate: pflags.contains(OpenFlags::TRUNCATE),
                    create: pflags.contains(OpenFlags::CREATE)
                        && !pflags.contains(OpenFlags::EXCLUDE),
                    create_new: pflags.contains(OpenFlags::EXCLUDE),
                },
            )
            .await
            .map_err(status_code)?;
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.application
            .close(self.session_id, handle)
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let data = self
            .application
            .read(self.session_id, handle, offset, len)
            .await
            .map_err(status_code)?;
        Ok(Data { id, data })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        self.application
            .write(self.session_id, handle, offset, data)
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: file_attributes(
                self.application
                    .lstat(self.session_id, path)
                    .await
                    .map_err(status_code)?,
            ),
        })
    }

    async fn fstat(&mut self, id: u32, handle: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: file_attributes(
                self.application
                    .fstat(self.session_id, handle)
                    .await
                    .map_err(status_code)?,
            ),
        })
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        self.application
            .setstat(self.session_id, path, server_attributes(attrs))
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn fsetstat(
        &mut self,
        id: u32,
        handle: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        self.application
            .fsetstat(self.session_id, handle, server_attributes(attrs))
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let handle = self
            .application
            .open_directory(self.session_id, path)
            .await
            .map_err(status_code)?;
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let entry = self
            .application
            .read_directory(self.session_id, handle)
            .await
            .map_err(status_code)?;
        Ok(Name {
            id,
            files: vec![SftpFile::new(entry.name, file_attributes(entry.metadata))],
        })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        self.application
            .remove(self.session_id, filename)
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        self.application
            .mkdir(self.session_id, path, server_attributes(attrs))
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        self.application
            .rmdir(self.session_id, path)
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let path = self
            .application
            .realpath(self.session_id, path)
            .await
            .map_err(status_code)?;
        Ok(Name {
            id,
            files: vec![SftpFile::dummy(path)],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: file_attributes(
                self.application
                    .stat(self.session_id, path)
                    .await
                    .map_err(status_code)?,
            ),
        })
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        self.application
            .rename(self.session_id, oldpath, newpath)
            .await
            .map_err(status_code)?;
        Ok(Self::status(id, StatusCode::Ok))
    }

    async fn readlink(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let target = self
            .application
            .read_link(self.session_id, path)
            .await
            .map_err(status_code)?;
        Ok(Name {
            id,
            files: vec![SftpFile::dummy(target)],
        })
    }
}

fn server_attributes(attributes: FileAttributes) -> SftpServerAttributes {
    SftpServerAttributes {
        size: attributes.size,
        permissions: attributes.permissions,
        atime: attributes.atime,
        mtime: attributes.mtime,
    }
}

fn file_attributes(metadata: SftpServerMetadata) -> FileAttributes {
    let mut attributes = FileAttributes {
        size: Some(metadata.size),
        permissions: Some(metadata.permissions),
        atime: Some(metadata.atime),
        mtime: Some(metadata.mtime),
        ..Default::default()
    };
    match metadata.file_type {
        SftpServerFileType::File => attributes.set_regular(true),
        SftpServerFileType::Directory => attributes.set_dir(true),
        SftpServerFileType::Symlink => attributes.set_symlink(true),
        SftpServerFileType::Other => {}
    }
    attributes
}

fn status_code(error: SftpServerError) -> StatusCode {
    match error {
        SftpServerError::ConnectionLost => StatusCode::ConnectionLost,
        SftpServerError::EndOfFile => StatusCode::Eof,
        SftpServerError::InvalidHandle | SftpServerError::NoSuchFile => StatusCode::NoSuchFile,
        SftpServerError::PermissionDenied => StatusCode::PermissionDenied,
        SftpServerError::Failure(message) => {
            log::debug!("sftp_server_request_failed error={message}");
            StatusCode::Failure
        }
    }
}
