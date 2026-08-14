mod core;
mod external;
mod internal;
mod ui;

use self::{core::default_desktop_path, ui::MultiSelection};

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, RwLock},
    time::{Instant, SystemTime},
};

use gpui::*;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, h_flex, v_flex};
use serde::Deserialize;
use tokio::{
    sync::{Notify, mpsc, oneshot},
    task::JoinHandle,
};

use crate::component::theme;

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
    request: TransferRequest,
    progress: f32,
    transferred_bytes: u64,
    total_bytes: u64,
    speed: u64,
    started_at: Option<Instant>,
    speed_updated_at: Option<Instant>,
    status: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
enum TransferRequest {
    Upload {
        workspace_id: String,
        local_path: PathBuf,
        is_directory: bool,
    },
    Download {
        workspace_id: String,
        remote_path: String,
        file_name: String,
        total_size: u64,
        is_directory: bool,
    },
}

impl TransferRequest {
    fn workspace_id(&self) -> &str {
        match self {
            Self::Upload { workspace_id, .. } | Self::Download { workspace_id, .. } => workspace_id,
        }
    }
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
    cancelled_transfers: RwLock<HashSet<u64>>,
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
    transfer_context_id: Option<u64>,
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let label = if self.paths.len() == 1 {
            self.paths[0]
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| self.paths[0].to_string_lossy().into_owned())
        } else {
            format!("{} 个本地项目", self.paths.len())
        };
        h_flex()
            .id("sftp-drag-preview-upload")
            .h(px(44.))
            .min_w(px(220.))
            .max_w(px(360.))
            .px_3()
            .gap_2()
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .shadow_lg()
            .child(
                Icon::new(IconName::ArrowUp)
                    .small()
                    .text_color(colors.primary),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap_0()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.primary)
                            .child("上传"),
                    )
                    .child(div().min_w_0().text_sm().child(label).truncate()),
            )
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

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct CancelTransfer(u64);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct RetryTransfer(u64);
impl Render for DragPreviewRemoteToLocalItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let label = if self.items.len() == 1 {
            self.items[0].name.clone()
        } else {
            format!("{} 个远程项目", self.items.len())
        };
        h_flex()
            .id("sftp-drag-preview-download")
            .h(px(44.))
            .min_w(px(220.))
            .max_w(px(360.))
            .px_3()
            .gap_2()
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .shadow_lg()
            .child(
                Icon::new(IconName::ArrowDown)
                    .small()
                    .text_color(colors.primary),
            )
            .child(
                v_flex()
                    .min_w_0()
                    .gap_0()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.primary)
                            .child("下载"),
                    )
                    .child(div().min_w_0().text_sm().child(label).truncate()),
            )
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
            transfer_context_id: None,
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
