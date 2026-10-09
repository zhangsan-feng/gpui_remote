use crate::infrastructure::INFRASTRUCTURE;
use std::{
    collections::HashMap,
    future::Future,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::Mutex;

use crate::application::sftp_server::{
    SftpServerAttributes, SftpServerDirectoryEntry, SftpServerError, SftpServerFileType,
    SftpServerHandleKind, SftpServerMetadata, SftpServerOpenOptions, SftpServerRawHandle,
};

#[derive(Clone)]
pub(crate) struct SftpServerApplication {
    sessions: Arc<StdMutex<HashMap<u64, Arc<Session>>>>,
    next_session: Arc<AtomicU64>,
}

impl SftpServerApplication {
    pub(crate) fn new() -> Self {
        Self {
            sessions: Arc::new(StdMutex::new(HashMap::new())),
            next_session: Arc::new(AtomicU64::new(0)),
        }
    }
}

struct Session {
    root: PathBuf,
    state: Mutex<SessionState>,
}

#[derive(Default)]
struct SessionState {
    handles: HashMap<String, ApplicationHandle>,
    next_handle: u64,
}

struct ApplicationHandle {
    path: PathBuf,
    raw: SftpServerRawHandle,
}

impl Session {
    async fn existing_path(&self, path: &str) -> Result<PathBuf, SftpServerError> {
        let candidate = lexical_path(&self.root, path)?;
        let resolved = INFRASTRUCTURE
            .sftp_server_canonicalize(candidate)
            .await
            .map_err(map_io_error)?;
        ensure_inside(&self.root, resolved)
    }

    async fn lstat_path(&self, path: &str) -> Result<PathBuf, SftpServerError> {
        let candidate = lexical_path(&self.root, path)?;
        if candidate == self.root {
            return Ok(candidate);
        }
        let parent = candidate
            .parent()
            .map(Path::to_owned)
            .unwrap_or_else(|| self.root.clone());
        let resolved_parent = INFRASTRUCTURE
            .sftp_server_canonicalize(parent)
            .await
            .map_err(map_io_error)?;
        ensure_inside(&self.root, resolved_parent)?;
        Ok(candidate)
    }

    async fn create_path(&self, path: &str) -> Result<PathBuf, SftpServerError> {
        let candidate = lexical_path(&self.root, path)?;
        if candidate == self.root {
            return Ok(candidate);
        }
        let parent = candidate
            .parent()
            .map(Path::to_owned)
            .unwrap_or_else(|| self.root.clone());
        let resolved_parent = INFRASTRUCTURE
            .sftp_server_canonicalize(parent)
            .await
            .map_err(map_io_error)?;
        ensure_inside(&self.root, resolved_parent)?;
        match INFRASTRUCTURE
            .sftp_server_canonicalize(candidate.clone())
            .await
        {
            Ok(resolved) => {
                ensure_inside(&self.root, resolved)?;
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(map_io_error(error)),
        }
        Ok(candidate)
    }

    async fn next_handle(&self, kind: &str, raw: SftpServerRawHandle, path: PathBuf) -> String {
        let mut state = self.state.lock().await;
        state.next_handle = state.next_handle.wrapping_add(1).max(1);
        let handle = format!("{kind}-{}", state.next_handle);
        state
            .handles
            .insert(handle.clone(), ApplicationHandle { path, raw });
        handle
    }

    async fn remove_handle(&self, handle: &str) -> Result<ApplicationHandle, SftpServerError> {
        self.state
            .lock()
            .await
            .handles
            .remove(handle)
            .ok_or(SftpServerError::InvalidHandle)
    }

    async fn raw_handle(
        &self,
        handle: &str,
        expected: SftpServerHandleKind,
    ) -> Result<SftpServerRawHandle, SftpServerError> {
        let state = self.state.lock().await;
        let value = state
            .handles
            .get(handle)
            .ok_or(SftpServerError::InvalidHandle)?;
        if std::mem::discriminant(&value.raw.kind) != std::mem::discriminant(&expected) {
            return Err(SftpServerError::Failure("SFTP 句柄类型不匹配".to_owned()));
        }
        Ok(value.raw)
    }

    async fn handle_path(&self, handle: &str) -> Result<PathBuf, SftpServerError> {
        self.state
            .lock()
            .await
            .handles
            .get(handle)
            .map(|value| value.path.clone())
            .ok_or(SftpServerError::InvalidHandle)
    }

    async fn close_all(&self) {
        let handles: Vec<_> = self
            .state
            .lock()
            .await
            .handles
            .drain()
            .map(|(_, value)| value.raw)
            .collect();
        for raw in handles {
            let _ = INFRASTRUCTURE.sftp_server_close(raw).await;
        }
    }
}

impl SftpServerApplication {
    pub(crate) fn open_session(&self, root: PathBuf) -> u64 {
        let session_id = self.next_session.fetch_add(1, Ordering::Relaxed).max(1);
        let session = Arc::new(Session {
            root,
            state: Mutex::new(SessionState::default()),
        });
        self.sessions
            .lock()
            .expect("SFTP session state lock poisoned")
            .insert(session_id, session);
        session_id
    }

    pub(crate) async fn close_session(&self, session_id: u64) -> Result<(), SftpServerError> {
        let sessions = self.sessions.clone();

        let session = sessions
            .lock()
            .map_err(|_| SftpServerError::Failure("SFTP 会话状态不可用".to_owned()))?
            .remove(&session_id)
            .ok_or(SftpServerError::ConnectionLost)?;
        session.close_all().await;
        Ok(())
    }

    pub(crate) async fn open(
        &self,
        session_id: u64,
        filename: String,
        options: SftpServerOpenOptions,
    ) -> Result<String, SftpServerError> {
        let sessions = self.sessions.clone();

        let session = sessions
            .lock()
            .map_err(|_| SftpServerError::Failure("SFTP 会话状态不可用".to_owned()))?
            .get(&session_id)
            .cloned()
            .ok_or(SftpServerError::ConnectionLost)?;
        let path = session.create_path(&filename).await?;
        let raw = INFRASTRUCTURE
            .sftp_server_open(path.clone(), options)
            .await
            .map_err(map_io_error)?;
        Ok(session.next_handle("file", raw, path).await)
    }

    pub(crate) async fn close(
        &self,
        session_id: u64,
        handle: String,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let value = session.remove_handle(&handle).await?;
        INFRASTRUCTURE
            .sftp_server_close(value.raw)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn read(
        &self,
        session_id: u64,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Vec<u8>, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let raw = session
            .raw_handle(&handle, SftpServerHandleKind::File)
            .await?;
        let data = INFRASTRUCTURE
            .sftp_server_read(raw, offset, len)
            .await
            .map_err(map_io_error)?;
        if data.is_empty() {
            return Err(SftpServerError::EndOfFile);
        }
        Ok(data)
    }

    pub(crate) async fn write(
        &self,
        session_id: u64,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let raw = session
            .raw_handle(&handle, SftpServerHandleKind::File)
            .await?;
        INFRASTRUCTURE
            .sftp_server_write(raw, offset, data)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn lstat(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<SftpServerMetadata, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.lstat_path(&path).await?;
        INFRASTRUCTURE
            .sftp_server_metadata(path, false)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn fstat(
        &self,
        session_id: u64,
        handle: String,
    ) -> Result<SftpServerMetadata, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let value = session
            .state
            .lock()
            .await
            .handles
            .get(&handle)
            .map(|value| (value.path.clone(), value.raw))
            .ok_or(SftpServerError::InvalidHandle)?;
        match value.1.kind {
            SftpServerHandleKind::File => INFRASTRUCTURE
                .sftp_server_metadata_handle(value.1)
                .await
                .map_err(map_io_error),
            SftpServerHandleKind::Directory => INFRASTRUCTURE
                .sftp_server_metadata(value.0, true)
                .await
                .map_err(map_io_error),
        }
    }

    pub(crate) async fn setstat(
        &self,
        session_id: u64,
        path: String,
        attributes: SftpServerAttributes,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        INFRASTRUCTURE
            .sftp_server_set_attributes(path, attributes)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn fsetstat(
        &self,
        session_id: u64,
        handle: String,
        attributes: SftpServerAttributes,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.handle_path(&handle).await?;
        INFRASTRUCTURE
            .sftp_server_set_attributes(path, attributes)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn open_directory(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<String, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        let metadata = INFRASTRUCTURE
            .sftp_server_metadata(path.clone(), true)
            .await
            .map_err(map_io_error)?;
        if !matches!(metadata.file_type, SftpServerFileType::Directory) {
            return Err(SftpServerError::Failure("目标路径不是目录".to_owned()));
        }
        let raw = INFRASTRUCTURE
            .sftp_server_open_directory(path.clone())
            .await
            .map_err(map_io_error)?;
        Ok(session.next_handle("directory", raw, path).await)
    }

    pub(crate) async fn read_directory(
        &self,
        session_id: u64,
        handle: String,
    ) -> Result<SftpServerDirectoryEntry, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let raw = session
            .raw_handle(&handle, SftpServerHandleKind::Directory)
            .await?;
        INFRASTRUCTURE
            .sftp_server_read_directory(raw)
            .await
            .map_err(map_io_error)?
            .ok_or(SftpServerError::EndOfFile)
    }

    pub(crate) async fn remove(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        reject_root(&session.root, &path)?;
        INFRASTRUCTURE
            .sftp_server_remove_file(path)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn mkdir(
        &self,
        session_id: u64,
        path: String,
        attributes: SftpServerAttributes,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.create_path(&path).await?;
        INFRASTRUCTURE
            .sftp_server_create_directory(path.clone())
            .await
            .map_err(map_io_error)?;
        if has_attributes(&attributes) {
            INFRASTRUCTURE
                .sftp_server_set_attributes(path, attributes)
                .await
                .map_err(map_io_error)?;
        }
        Ok(())
    }

    pub(crate) async fn rmdir(&self, session_id: u64, path: String) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        reject_root(&session.root, &path)?;
        INFRASTRUCTURE
            .sftp_server_remove_directory(path)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn realpath(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<String, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        virtual_path(&session.root, &path)
    }

    pub(crate) async fn stat(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<SftpServerMetadata, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.existing_path(&path).await?;
        INFRASTRUCTURE
            .sftp_server_metadata(path, true)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn rename(
        &self,
        session_id: u64,
        old_path: String,
        new_path: String,
    ) -> Result<(), SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let old_path = session.existing_path(&old_path).await?;
        let new_path = session.create_path(&new_path).await?;
        reject_root(&session.root, &old_path)?;
        reject_root(&session.root, &new_path)?;
        INFRASTRUCTURE
            .sftp_server_rename(old_path, new_path)
            .await
            .map_err(map_io_error)
    }

    pub(crate) async fn read_link(
        &self,
        session_id: u64,
        path: String,
    ) -> Result<String, SftpServerError> {
        let session = self.session_future(session_id);

        let session = session.await?;
        let path = session.lstat_path(&path).await?;
        INFRASTRUCTURE
            .sftp_server_read_link(path)
            .await
            .map(|target| target.to_string_lossy().into_owned())
            .map_err(map_io_error)
    }
}

impl SftpServerApplication {
    fn session_future(
        &self,
        session_id: u64,
    ) -> impl Future<Output = Result<Arc<Session>, SftpServerError>> + Send {
        let sessions = self.sessions.clone();
        async move {
            sessions
                .lock()
                .map_err(|_| SftpServerError::Failure("SFTP 会话状态不可用".to_owned()))
                .and_then(|sessions| {
                    sessions
                        .get(&session_id)
                        .cloned()
                        .ok_or(SftpServerError::ConnectionLost)
                })
        }
    }
}

fn lexical_path(root: &Path, path: &str) -> Result<PathBuf, SftpServerError> {
    let mut relative = PathBuf::new();
    for component in path.split(|character| character == '/' || character == '\\') {
        match component {
            "" | "." => {}
            ".." => {
                if !relative.pop() {
                    return Err(SftpServerError::PermissionDenied);
                }
            }
            component => relative.push(component),
        }
    }
    Ok(root.join(relative))
}

fn ensure_inside(root: &Path, path: PathBuf) -> Result<PathBuf, SftpServerError> {
    if path.starts_with(root) {
        Ok(path)
    } else {
        Err(SftpServerError::PermissionDenied)
    }
}

fn virtual_path(root: &Path, path: &Path) -> Result<String, SftpServerError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| SftpServerError::PermissionDenied)?;
    let mut result = String::from("/");
    for component in relative.components() {
        if let Component::Normal(component) = component {
            if result.len() > 1 {
                result.push('/');
            }
            result.push_str(&component.to_string_lossy());
        }
    }
    Ok(result)
}

fn reject_root(root: &Path, path: &Path) -> Result<(), SftpServerError> {
    if path == root {
        Err(SftpServerError::PermissionDenied)
    } else {
        Ok(())
    }
}

fn has_attributes(attributes: &SftpServerAttributes) -> bool {
    attributes.permissions.is_some()
        || attributes.atime.is_some()
        || attributes.mtime.is_some()
        || attributes.size.is_some()
}

fn map_io_error(error: std::io::Error) -> SftpServerError {
    log::debug!("sftp_server_filesystem_failed error={error}");
    match error.kind() {
        ErrorKind::NotFound => SftpServerError::NoSuchFile,
        ErrorKind::PermissionDenied => SftpServerError::PermissionDenied,
        _ => SftpServerError::Failure(error.to_string()),
    }
}
