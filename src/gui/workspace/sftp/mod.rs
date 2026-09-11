mod core;
mod external;
mod internal;
mod ui;

use self::ui::MultiSelection;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, h_flex, v_flex};
use gpui_kit::*;
use serde::Deserialize;
use tokio::sync::Notify;

use crate::{
    application::{ApplicationContext, model::SftpWatchSummary},
    component::theme,
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
    workspace_id: String,
    name: String,
    direction: String,
    target: String,
    progress: f32,
    speed: u64,
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
}

struct SftpProjection {
    profile_id: String,
    profile_ip: String,
    profile_title: String,
    model: Arc<SftpModel>,
}

pub(in crate::gui::workspace) struct SftpView {
    projections: HashMap<String, SftpProjection>,
    local_watchers: HashMap<String, HashMap<PathBuf, SftpWatchSummary>>,
    remote_revisions: HashMap<String, u64>,
    local_restore_requests: HashSet<String>,
    persisted_remote_paths: HashMap<String, String>,
    selected_workspace_id: Option<String>,
    local: LocalSnapshot,
    local_context_path: Option<PathBuf>,
    local_selection: MultiSelection<PathBuf>,
    drag_started: bool,
    remote_context_entry: Option<(String, bool)>,
    remote_selection: MultiSelection<String>,
    transfers: Arc<RwLock<Vec<TransferRecord>>>,
    transfer_context_id: Option<u64>,
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

#[derive(Clone, PartialEq, Eq, Deserialize)]
struct RemoteDeleteItem {
    path: String,
    is_directory: bool,
}

#[derive(Clone)]
struct DragPreviewRemoteToLocalItem {
    items: Vec<RemoteTransferItem>,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct DeleteLocalEntry(Vec<PathBuf>);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct DeleteRemoteEntry {
    items: Vec<RemoteDeleteItem>,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct UploadLocalEntry(Vec<PathBuf>);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct WatchLocalPath(PathBuf);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = sftp, no_json)]
struct StopWatchingLocalPath(PathBuf);

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
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let updates = application.sftp_updates();
        let status_updates = application.sftp_status_updates();
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
                let result = this.update(cx, |this, cx| {
                    this.sync_application_state(cx);
                    cx.notify();
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        let this = Self {
            projections: HashMap::new(),
            local_watchers: HashMap::new(),
            remote_revisions: HashMap::new(),
            local_restore_requests: HashSet::new(),
            persisted_remote_paths: HashMap::new(),
            selected_workspace_id: None,
            local: LocalSnapshot {
                path: PathBuf::new(),
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
            local_list_state,
            remote_list_state,
            transfer_list_state,
            updates,
            status_updates,
        };
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
        self.stop_all_local_watchers();
    }
}
