#[derive(Debug)]
pub(crate) enum SftpServerError {
    ConnectionLost,
    EndOfFile,
    InvalidHandle,
    NoSuchFile,
    PermissionDenied,
    Failure(String),
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SftpServerOpenOptions {
    pub(crate) read: bool,
    pub(crate) write: bool,
    pub(crate) append: bool,
    pub(crate) truncate: bool,
    pub(crate) create: bool,
    pub(crate) create_new: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SftpServerAttributes {
    pub(crate) size: Option<u64>,
    pub(crate) permissions: Option<u32>,
    pub(crate) atime: Option<u32>,
    pub(crate) mtime: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SftpServerFileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SftpServerMetadata {
    pub(crate) size: u64,
    pub(crate) file_type: SftpServerFileType,
    pub(crate) permissions: u32,
    pub(crate) atime: u32,
    pub(crate) mtime: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct SftpServerDirectoryEntry {
    pub(crate) name: String,
    pub(crate) metadata: SftpServerMetadata,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SftpServerHandleKind {
    File,
    Directory,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SftpServerRawHandle {
    pub(crate) id: u64,
    pub(crate) kind: SftpServerHandleKind,
}
