use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpSettings {
    pub enabled: bool,
    pub token_enabled: bool,
    pub host: String,
    pub port: u16,
    pub token: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub title: String,
    pub host: String,
    pub protocol: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TerminalSummary {
    pub workspace_id: String,
    pub profile_id: String,
    pub ip: String,
    pub title: String,
    pub host: String,
    pub protocol: String,
    pub status: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TerminalReadPage {
    pub workspace_id: String,
    pub text: String,
    pub total_lines: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    pub revision: u64,
    pub changed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpEntrySummary {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified_at: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct SftpDirectorySummary {
    pub path: String,
    pub entries: Vec<SftpEntrySummary>,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpTransferSummary {
    pub queued: usize,
    pub transfers: Vec<SftpTransferInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpTransferInfo {
    pub id: u64,
    pub workspace_id: String,
    pub name: String,
    pub direction: String,
    pub source: String,
    pub target: String,
    pub is_directory: bool,
    pub progress: f32,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_second: u64,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpWatchSummary {
    pub workspace_id: String,
    pub ip: String,
    pub title: String,
    pub local_path: String,
    pub remote_path: String,
    pub is_directory: bool,
    pub debounce_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SftpWorkspaceSnapshot {
    pub remote: SftpDirectorySummary,
    pub local: SftpDirectorySummary,
    pub transfers: Vec<SftpTransferInfo>,
    pub watches: Vec<SftpWatchSummary>,
    pub status: String,
    pub remote_revision: u64,
}

impl Default for SftpWorkspaceSnapshot {
    fn default() -> Self {
        Self {
            remote: SftpDirectorySummary {
                loading: true,
                ..SftpDirectorySummary::default()
            },
            local: SftpDirectorySummary {
                loading: true,
                ..SftpDirectorySummary::default()
            },
            transfers: Vec::new(),
            watches: Vec::new(),
            status: "connecting".to_owned(),
            remote_revision: 0,
        }
    }
}
