use std::{
    collections::HashMap,
    fs::Metadata,
    io::ErrorKind,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use tokio::{
    fs::{self, File, OpenOptions, ReadDir},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::Mutex,
};

use crate::application::sftp_server::{
    SftpServerAttributes, SftpServerDirectoryEntry, SftpServerFileType, SftpServerHandleKind,
    SftpServerMetadata, SftpServerOpenOptions, SftpServerRawHandle,
};

#[derive(Clone)]
pub(crate) struct LocalSftpFilesystem {
    handles: Arc<Mutex<HashMap<u64, Arc<RawHandle>>>>,
    next_handle: Arc<std::sync::atomic::AtomicU64>,
}

enum RawHandle {
    File { file: Mutex<File> },
    Directory { entries: Mutex<ReadDir> },
}

impl LocalSftpFilesystem {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
            next_handle: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        })
    }

    async fn insert_handle(
        &self,
        kind: SftpServerHandleKind,
        raw: RawHandle,
    ) -> SftpServerRawHandle {
        let id = self
            .next_handle
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .max(1);
        self.handles.lock().await.insert(id, Arc::new(raw));
        SftpServerRawHandle { id, kind }
    }

    async fn get_handle(&self, handle: SftpServerRawHandle) -> std::io::Result<Arc<RawHandle>> {
        self.handles
            .lock()
            .await
            .get(&handle.id)
            .cloned()
            .ok_or_else(|| std::io::Error::new(ErrorKind::NotFound, "SFTP raw handle not found"))
    }
}

impl LocalSftpFilesystem {
    pub(crate) async fn canonicalize(&self, path: PathBuf) -> std::io::Result<PathBuf> {
        fs::canonicalize(path).await
    }

    pub(crate) async fn open(
        &self,
        path: PathBuf,
        options: SftpServerOpenOptions,
    ) -> std::io::Result<SftpServerRawHandle> {
        let filesystem = self.clone();

        let mut open_options = OpenOptions::new();
        open_options
            .read(options.read)
            .write(options.write)
            .append(options.append)
            .truncate(options.truncate)
            .create(options.create)
            .create_new(options.create_new);
        let file = open_options.open(path).await?;
        Ok(filesystem
            .insert_handle(
                SftpServerHandleKind::File,
                RawHandle::File {
                    file: Mutex::new(file),
                },
            )
            .await)
    }

    pub(crate) async fn open_directory(
        &self,
        path: PathBuf,
    ) -> std::io::Result<SftpServerRawHandle> {
        let filesystem = self.clone();

        let entries = fs::read_dir(path).await?;
        Ok(filesystem
            .insert_handle(
                SftpServerHandleKind::Directory,
                RawHandle::Directory {
                    entries: Mutex::new(entries),
                },
            )
            .await)
    }

    pub(crate) async fn close(&self, handle: SftpServerRawHandle) -> std::io::Result<()> {
        let filesystem = self.clone();

        filesystem
            .handles
            .lock()
            .await
            .remove(&handle.id)
            .map(|_| ())
            .ok_or_else(|| std::io::Error::new(ErrorKind::NotFound, "SFTP raw handle not found"))
    }

    pub(crate) async fn read(
        &self,
        handle: SftpServerRawHandle,
        offset: u64,
        len: u32,
    ) -> std::io::Result<Vec<u8>> {
        let filesystem = self.clone();

        let raw = filesystem.get_handle(handle).await?;
        let RawHandle::File { file } = raw.as_ref() else {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "SFTP handle is not a file",
            ));
        };
        let mut file = file.lock().await;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let mut data = vec![0; len as usize];
        let read = file.read(&mut data).await?;
        data.truncate(read);
        Ok(data)
    }

    pub(crate) async fn write(
        &self,
        handle: SftpServerRawHandle,
        offset: u64,
        data: Vec<u8>,
    ) -> std::io::Result<()> {
        let filesystem = self.clone();

        let raw = filesystem.get_handle(handle).await?;
        let RawHandle::File { file } = raw.as_ref() else {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "SFTP handle is not a file",
            ));
        };
        let mut file = file.lock().await;
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.write_all(&data).await?;
        file.flush().await
    }

    pub(crate) async fn metadata(
        &self,
        path: PathBuf,
        follow_links: bool,
    ) -> std::io::Result<SftpServerMetadata> {
        let metadata = if follow_links {
            fs::metadata(path).await?
        } else {
            fs::symlink_metadata(path).await?
        };
        Ok(metadata_from_std(&metadata))
    }

    pub(crate) async fn metadata_handle(
        &self,
        handle: SftpServerRawHandle,
    ) -> std::io::Result<SftpServerMetadata> {
        let filesystem = self.clone();

        let raw = filesystem.get_handle(handle).await?;
        let metadata = match raw.as_ref() {
            RawHandle::File { file } => file.lock().await.metadata().await?,
            RawHandle::Directory { .. } => {
                return Err(std::io::Error::new(
                    ErrorKind::Unsupported,
                    "directory handle metadata requires path",
                ));
            }
        };
        Ok(metadata_from_std(&metadata))
    }

    pub(crate) async fn read_directory(
        &self,
        handle: SftpServerRawHandle,
    ) -> std::io::Result<Option<SftpServerDirectoryEntry>> {
        let filesystem = self.clone();

        let raw = filesystem.get_handle(handle).await?;
        let RawHandle::Directory { entries } = raw.as_ref() else {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "SFTP handle is not a directory",
            ));
        };
        let mut entries = entries.lock().await;
        let Some(entry) = entries.next_entry().await? else {
            return Ok(None);
        };
        Ok(Some(SftpServerDirectoryEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            metadata: metadata_from_std(&entry.metadata().await?),
        }))
    }

    pub(crate) async fn set_attributes(
        &self,
        path: PathBuf,
        attributes: SftpServerAttributes,
    ) -> std::io::Result<()> {
        apply_attributes(path, attributes).await
    }

    pub(crate) async fn create_directory(&self, path: PathBuf) -> std::io::Result<()> {
        fs::create_dir(path).await
    }

    pub(crate) async fn remove_file(&self, path: PathBuf) -> std::io::Result<()> {
        fs::remove_file(path).await
    }

    pub(crate) async fn remove_directory(&self, path: PathBuf) -> std::io::Result<()> {
        fs::remove_dir(path).await
    }

    pub(crate) async fn rename(&self, old_path: PathBuf, new_path: PathBuf) -> std::io::Result<()> {
        fs::rename(old_path, new_path).await
    }

    pub(crate) async fn read_link(&self, path: PathBuf) -> std::io::Result<PathBuf> {
        fs::read_link(path).await
    }
}

fn metadata_from_std(metadata: &Metadata) -> SftpServerMetadata {
    let file_type = if metadata.file_type().is_symlink() {
        SftpServerFileType::Symlink
    } else if metadata.is_dir() {
        SftpServerFileType::Directory
    } else if metadata.is_file() {
        SftpServerFileType::File
    } else {
        SftpServerFileType::Other
    };
    SftpServerMetadata {
        size: metadata.len(),
        file_type,
        permissions: permissions_from_std(metadata),
        atime: unix_seconds(metadata.accessed().ok()),
        mtime: unix_seconds(metadata.modified().ok()),
    }
}

fn permissions_from_std(metadata: &Metadata) -> u32 {
    #[cfg(unix)]
    {
        metadata.mode()
    }
    #[cfg(windows)]
    {
        if metadata.permissions().readonly() {
            0o555
        } else {
            0o777
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        0o777
    }
}

fn unix_seconds(time: Option<SystemTime>) -> u32 {
    time.and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or_default()
}

async fn apply_attributes(path: PathBuf, attributes: SftpServerAttributes) -> std::io::Result<()> {
    if let Some(size) = attributes.size {
        let file = OpenOptions::new().write(true).open(&path).await?;
        file.set_len(size).await?;
    }
    if attributes.atime.is_none() && attributes.mtime.is_none() && attributes.permissions.is_none()
    {
        return Ok(());
    }
    tokio::task::spawn_blocking(move || {
        let metadata = std::fs::symlink_metadata(&path)?;
        let access_time = attributes
            .atime
            .map(|seconds| filetime::FileTime::from_unix_time(seconds as i64, 0))
            .unwrap_or_else(|| filetime::FileTime::from_last_access_time(&metadata));
        let modified_time = attributes
            .mtime
            .map(|seconds| filetime::FileTime::from_unix_time(seconds as i64, 0))
            .unwrap_or_else(|| filetime::FileTime::from_last_modification_time(&metadata));
        filetime::set_file_times(&path, access_time, modified_time)?;

        #[cfg(unix)]
        if let Some(permissions) = attributes.permissions {
            use std::os::unix::fs::PermissionsExt;
            let mut value = metadata.permissions();
            value.set_mode(permissions & 0o7777);
            std::fs::set_permissions(&path, value)?;
        }
        #[cfg(windows)]
        if let Some(permissions) = attributes.permissions {
            let mut value = metadata.permissions();
            value.set_readonly(permissions & 0o222 == 0);
            std::fs::set_permissions(&path, value)?;
        }
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|error| std::io::Error::new(ErrorKind::Other, error))??;
    Ok(())
}
