use std::path::{Path, PathBuf};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};

use super::{SftpApplication, remote};

const WATCH_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalWatchKind {
    File,
    Directory,
}

#[derive(Clone, Debug)]
pub(crate) struct LocalWatchSummary {
    pub(crate) workspace_id: String,
    pub(crate) ip: String,
    pub(crate) title: String,
    pub(crate) local_path: String,
    pub(crate) remote_path: String,
    pub(crate) is_directory: bool,
    pub(crate) debounce_ms: u64,
}

pub(crate) struct LocalWatchRuntime {
    pub(crate) summary: LocalWatchSummary,
    pub(crate) stop: Option<oneshot::Sender<()>>,
    pub(crate) task: tokio::task::JoinHandle<()>,
}

impl LocalWatchRuntime {
    pub(crate) fn stop(mut self) {
        log::debug!(
            "SFTP local watcher stopping: workspace_id={}, local_path={}",
            self.summary.workspace_id,
            self.summary.local_path
        );
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
    }
}

pub(crate) async fn listen_local_directory(
    app: SftpApplication,
    workspace_id: String,
    ip: String,
    title: String,
    local_path: PathBuf,
    remote_path: String,
) -> Result<LocalWatchRuntime, String> {
    log::debug!(
        "SFTP local watcher create started: workspace_id={workspace_id}, local_path={}, remote_path={remote_path}",
        local_path.display()
    );
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<notify::Result<Event>>();
    let (stop_sender, mut stop_receiver) = oneshot::channel();
    let setup_path = local_path.clone();
    let (watcher, kind) = tokio::task::spawn_blocking(move || {
        let metadata = std::fs::metadata(&setup_path)
            .map_err(|error| format!("监听本地路径不存在: {}: {error}", setup_path.display()))?;
        let kind = if metadata.is_dir() {
            LocalWatchKind::Directory
        } else if metadata.is_file() {
            LocalWatchKind::File
        } else {
            return Err(format!("不支持监听本地路径类型: {}", setup_path.display()));
        };
        let watch_root = watch_root(&setup_path, kind);
        let recursive_mode = match kind {
            LocalWatchKind::File => RecursiveMode::NonRecursive,
            LocalWatchKind::Directory => RecursiveMode::Recursive,
        };
        let callback_sender = event_sender;
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                let _ = callback_sender.send(result);
            },
            Config::default(),
        )
        .map_err(|error| format!("创建监听器失败: {error:#}"))?;
        watcher
            .watch(&watch_root, recursive_mode)
            .map_err(|error| format!("监听路径失败 {}: {error:#}", watch_root.display()))?;
        Ok::<_, String>((watcher, kind))
    })
    .await
    .map_err(|error| format!("创建本地监听任务失败: {error}"))??;

    let task_workspace_id = workspace_id.clone();
    let task_local_root = local_path.clone();
    let task_remote_root = remote_path.clone();
    let task_app = app.clone();
    let task = tokio::spawn(async move {
        let _watcher = watcher;
        let mut pending_paths = std::collections::HashSet::new();
        loop {
            let Some(event) = (tokio::select! {
                _ = &mut stop_receiver => return,
                event = event_receiver.recv() => event,
            }) else {
                return;
            };
            collect_event_paths(event, kind, &task_local_root, &mut pending_paths);
            loop {
                let sleep = tokio::time::sleep(WATCH_DEBOUNCE);
                tokio::pin!(sleep);
                tokio::select! {
                    _ = &mut stop_receiver => return,
                    event = event_receiver.recv() => {
                        let Some(event) = event else { return; };
                        collect_event_paths(event, kind, &task_local_root, &mut pending_paths);
                    }
                    _ = &mut sleep => break,
                }
            }
            let paths = std::mem::take(&mut pending_paths);
            let mut paths = paths.into_iter().collect::<Vec<_>>();
            paths.sort_by_key(|path| path.components().count());
            let mut uploaded_roots = Vec::new();
            for path in paths {
                if kind == LocalWatchKind::Directory
                    && uploaded_roots
                        .iter()
                        .any(|root: &PathBuf| path.starts_with(root))
                {
                    continue;
                }
                let target = match kind {
                    LocalWatchKind::File => task_remote_root.clone(),
                    LocalWatchKind::Directory => {
                        let relative = path
                            .strip_prefix(&task_local_root)
                            .map(|path| path.to_string_lossy().replace('\\', "/"))
                            .unwrap_or_default();
                        if relative.is_empty() {
                            task_remote_root.clone()
                        } else {
                            remote::join_remote_path(&task_remote_root, &relative)
                        }
                    }
                };
                if task_app
                    .upload_path_to_remote(&task_workspace_id, path.clone(), target)
                    .is_some()
                    && kind == LocalWatchKind::Directory
                {
                    uploaded_roots.push(path);
                }
            }
        }
    });

    Ok(LocalWatchRuntime {
        summary: LocalWatchSummary {
            workspace_id,
            ip,
            title,
            local_path: local_path.display().to_string(),
            remote_path,
            is_directory: kind == LocalWatchKind::Directory,
            debounce_ms: WATCH_DEBOUNCE.as_millis() as u64,
        },
        stop: Some(stop_sender),
        task,
    })
}

fn watch_root(path: &Path, kind: LocalWatchKind) -> PathBuf {
    match kind {
        LocalWatchKind::File => path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_owned)
            .unwrap_or_else(|| PathBuf::from(".")),
        LocalWatchKind::Directory => path.to_owned(),
    }
}

fn collect_event_paths(
    result: notify::Result<Event>,
    kind: LocalWatchKind,
    local_root: &Path,
    pending_paths: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(event) = result.inspect_err(|error| log::error!("本地自动上传监听异常: {error:#}"))
    else {
        return;
    };
    if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
        return;
    }
    for path in event.paths {
        let relevant = match kind {
            LocalWatchKind::File => same_file_path(&path, local_root),
            LocalWatchKind::Directory => path == local_root || path.starts_with(local_root),
        };
        if relevant {
            pending_paths.insert(path);
        }
    }
}

fn same_file_path(left: &Path, right: &Path) -> bool {
    left == right || (left.file_name() == right.file_name() && left.parent() == right.parent())
}
