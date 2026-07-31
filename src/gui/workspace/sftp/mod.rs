mod core;
mod external;
mod internal;
mod ui;

use self::core::default_desktop_path;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use gpui::*;
use tokio::{
    sync::{Notify, mpsc},
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
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

enum SftpCommand {
    LoadDirectory(String),
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
    transfers: Vec<TransferRecord>,
    local_list_state: ListState,
    remote_list_state: ListState,
    transfer_list_state: ListState,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
}

struct DragPreviewItem {
    path: PathBuf,
}
impl Render for DragPreviewItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.path.to_str().unwrap().to_string())
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
            transfers: Vec::new(),
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
