mod core;
mod external;
mod internal;
mod ui;

use self::{core::default_desktop_path, ui::MultiSelection};

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use gpui::*;
use serde::Deserialize;
use tokio::{
    sync::{Notify, mpsc, oneshot},
    task::JoinHandle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SftpStatus {
    Connecting,
    Connected,
    Disconnected,
    Failed,
}

#[derive(Clone, Debug)]
struct SftpEntry {
    name: String,
    path: String,
    is_directory: bool,
    size: u64,
    modified_at: Option<u32>,
}

#[derive(Clone, Debug)]
struct LocalEntry {
    name: String,
    path: PathBuf,
    is_directory: bool,
    size: u64,
    modified_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
struct LocalSnapshot {
    path: PathBuf,
    entries: Arc<Vec<LocalEntry>>,
    loading: bool,
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct TransferRecord {
    id: u64,
    name: String,
    direction: String,
    target: String,
    progress: f32,
    status: String,
}

#[derive(Clone, Debug)]
struct SftpSnapshot {
    status: SftpStatus,
    path: String,
    entries: Arc<Vec<SftpEntry>>,
    loading: bool,
    error: Option<String>,
}

impl Default for SftpSnapshot {
    fn default() -> Self {
        Self {
            status: SftpStatus::Connecting,
            path: String::new(),
            entries: Arc::new(Vec::new()),
            loading: true,
            error: None,
        }
    }
}

struct SftpModel {
    snapshot: RwLock<SftpSnapshot>,
    transfers: Arc<RwLock<Vec<TransferRecord>>>,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

enum SftpCommand {
    LoadDirectory(String),
    Upload {
        transfer_id: u64,
        local_path: PathBuf,
        remote_path: String,
        refresh_path: String,
    },
    Download {
        transfer_id: u64,
        remote_path: String,
        local_path: PathBuf,
        total_size: u64,
        is_directory: bool,
        complete: oneshot::Sender<bool>,
    },
    Delete {
        path: String,
        is_directory: bool,
        refresh_path: String,
    },
    Disconnect,
}

struct SftpRuntime {
    model: Arc<SftpModel>,
    commands: mpsc::UnboundedSender<SftpCommand>,
    task: JoinHandle<()>,
}

pub(in crate::gui::workspace) struct SftpView {
    runtimes: HashMap<String, SftpRuntime>,
    selected_workspace_id: Option<String>,
    local: LocalSnapshot,
    local_context_path: Option<PathBuf>,
    local_selection: MultiSelection<PathBuf>,
    drag_started: bool,
    remote_context_entry: Option<(String, bool)>,
    remote_selection: MultiSelection<String>,
    transfers: Arc<RwLock<Vec<TransferRecord>>>,
    next_transfer_id: u64,
    local_list_state: ListState,
    remote_list_state: ListState,
    transfer_list_state: ListState,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

#[derive(Clone)]
struct DragPreviewLocalToRemoteItem {
    paths: Vec<PathBuf>,
}
impl Render for DragPreviewLocalToRemoteItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.paths.len() == 1 {
            self.paths[0].to_string_lossy().into_owned()
        } else {
            format!("{} 个本地项目", self.paths.len())
        };
        div().size_full().child(label)
    }
}

#[derive(Clone, PartialEq, Eq, Deserialize)]
struct RemoteTransferItem {
    path: String,
    name: String,
    size: u64,
    is_directory: bool,
}

#[derive(Clone)]
struct DragPreviewRemoteToLocalItem {
    items: Vec<RemoteTransferItem>,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct DeleteLocalEntry(PathBuf);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct DeleteRemoteEntry {
    path: String,
    is_directory: bool,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct UploadLocalEntry(Vec<PathBuf>);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct DownloadRemoteEntry {
    items: Vec<RemoteTransferItem>,
}
impl Render for DragPreviewRemoteToLocalItem {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.items.len() == 1 {
            self.items[0].name.clone()
        } else {
            format!("{} 个远程项目", self.items.len())
        };
        div().size_full().child(label)
    }
}

impl SftpView {
    pub(in crate::gui::workspace) fn new(cx: &mut Context<Self>) -> Self {
        let updates = Arc::new(Notify::new());
        let status_updates = Arc::new(Notify::new());
        let local_list_state =
            ListState::new(0, ListAlignment::Top, px(256.)).with_uniform_item_height(px(38.));
        let remote_list_state =
            ListState::new(0, ListAlignment::Top, px(256.)).with_uniform_item_height(px(38.));
        let transfer_list_state =
            ListState::new(0, ListAlignment::Top, px(256.)).with_uniform_item_height(px(38.));
        let model_updates = updates.clone();
        cx.spawn(async move |this, cx| {
            loop {
                model_updates.notified().await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();

        let mut this = Self {
            runtimes: HashMap::new(),
            selected_workspace_id: None,
            local: LocalSnapshot {
                path: default_desktop_path(),
                entries: Arc::new(Vec::new()),
                loading: true,
                error: None,
            },
            local_context_path: None,
            local_selection: MultiSelection::default(),
            drag_started: false,
            remote_context_entry: None,
            remote_selection: MultiSelection::default(),
            transfers: Arc::new(RwLock::new(Vec::new())),
            next_transfer_id: 1,
            local_list_state,
            remote_list_state,
            transfer_list_state,
            updates,
            status_updates,
        };
        this.load_local_directory(this.local.path.clone(), cx);
        this.start_subscribe(cx);
        this
    }
}

impl Render for SftpView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}

impl Drop for SftpView {
    fn drop(&mut self) {
        for (_, runtime) in self.runtimes.drain() {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }
}
