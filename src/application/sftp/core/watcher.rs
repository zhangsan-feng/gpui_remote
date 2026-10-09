use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use tokio::sync::oneshot;

use crate::application::ports::LocalWatchSource;
use crate::infrastructure::INFRASTRUCTURE;

use super::{SftpApplication, remote};
use crate::domain::sftp::FileSignature;

pub(crate) type LocalWatchSummary = crate::data_context::SftpWatchSummary;

const WATCH_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(2);
const STABILITY_CHECK_DELAY: std::time::Duration = std::time::Duration::from_millis(250);
const STABILITY_CHECK_RETRIES: usize = 3;

enum LocalSignatureState {
    Stable(FileSignature),
    Missing,
    Unstable(FileSignature),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalWatchKind {
    File,
    Directory,
}

pub(crate) struct LocalWatchRuntime {
    pub(crate) workspace_id: String,
    pub(crate) local_path: PathBuf,
    pub(crate) stop: Option<oneshot::Sender<()>>,
    pub(crate) task: tokio::task::JoinHandle<()>,
}

impl LocalWatchRuntime {
    pub(crate) fn stop(mut self) {
        log::debug!(
            "SFTP local watcher stopping: workspace_id={}, local_path={}",
            self.workspace_id,
            self.local_path.display()
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
) -> Result<(LocalWatchRuntime, LocalWatchSummary), String> {
    log::debug!(
        "SFTP local watcher create started: workspace_id={workspace_id}, local_path={}, remote_path={remote_path}",
        local_path.display()
    );
    let (stop_sender, mut stop_receiver) = oneshot::channel();
    let LocalWatchSource {
        mut events,
        is_directory,
        watcher,
    } = INFRASTRUCTURE
        .watch_path(local_path.clone())
        .await
        .map_err(|error| format!("{error:#}"))?;
    let kind = if is_directory {
        LocalWatchKind::Directory
    } else {
        LocalWatchKind::File
    };

    let task_workspace_id = workspace_id.clone();
    let task_local_root = local_path.clone();
    let task_remote_root = remote_path.clone();
    let task_app = app.clone();
    let task = tokio::spawn(async move {
        let _watcher = watcher;
        let mut pending_paths = HashSet::new();
        let mut processed_signatures = HashMap::<PathBuf, FileSignature>::new();
        loop {
            if pending_paths.is_empty() {
                let Some(event) = (tokio::select! {
                    _ = &mut stop_receiver => return,
                    event = events.recv() => event,
                }) else {
                    return;
                };
                collect_event_paths(event, &mut pending_paths);
                loop {
                    let sleep = tokio::time::sleep(WATCH_DEBOUNCE);
                    tokio::pin!(sleep);
                    tokio::select! {
                        _ = &mut stop_receiver => return,
                        event = events.recv() => {
                            let Some(event) = event else { return; };
                            collect_event_paths(event, &mut pending_paths);
                        }
                        _ = &mut sleep => break,
                    }
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

                let signature = match read_stable_local_signature(&path).await {
                    Ok(LocalSignatureState::Stable(signature)) => signature,
                    Ok(LocalSignatureState::Missing) => {
                        processed_signatures.remove(&path);
                        continue;
                    }
                    Ok(LocalSignatureState::Unstable(signature)) => {
                        log::debug!(
                            "SFTP watcher 文件仍在写入，稍后重试: path={}, signature={signature:?}",
                            path.display()
                        );
                        pending_paths.insert(path);
                        continue;
                    }
                    Err(error) => {
                        log::warn!(
                            "SFTP watcher 无法读取路径元数据: path={}, error={error}",
                            path.display()
                        );
                        processed_signatures.remove(&path);
                        continue;
                    }
                };

                if processed_signatures
                    .get(&path)
                    .is_some_and(|previous| *previous == signature)
                {
                    log::debug!(
                        "SFTP watcher 跳过未变化路径: workspace_id={}, path={}",
                        task_workspace_id,
                        path.display()
                    );
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
                let Some(completion) =
                    task_app.upload_path_to_remote(&task_workspace_id, path.clone(), target)
                else {
                    continue;
                };
                if completion.await.unwrap_or(false) {
                    processed_signatures.insert(path.clone(), signature);
                    if kind == LocalWatchKind::Directory {
                        uploaded_roots.push(path);
                    }
                }
            }
        }
    });

    let summary = LocalWatchSummary {
        workspace_id,
        ip,
        title,
        local_path: local_path.display().to_string(),
        remote_path,
        is_directory: kind == LocalWatchKind::Directory,
        debounce_ms: WATCH_DEBOUNCE.as_millis() as u64,
    };
    Ok((
        LocalWatchRuntime {
            workspace_id: summary.workspace_id.clone(),
            local_path,
            stop: Some(stop_sender),
            task,
        },
        summary,
    ))
}

fn collect_event_paths(
    result: Result<Vec<PathBuf>, String>,
    pending_paths: &mut std::collections::HashSet<PathBuf>,
) {
    let Ok(event) = result.inspect_err(|error| log::error!("本地自动上传监听异常: {error:#}"))
    else {
        return;
    };
    for path in event {
        pending_paths.insert(path);
    }
}

async fn read_stable_local_signature(path: &Path) -> Result<LocalSignatureState, String> {
    let Some(mut signature) = read_local_signature(path).await? else {
        return Ok(LocalSignatureState::Missing);
    };

    for _ in 0..STABILITY_CHECK_RETRIES {
        tokio::time::sleep(STABILITY_CHECK_DELAY).await;
        let Some(current_signature) = read_local_signature(path).await? else {
            return Ok(LocalSignatureState::Missing);
        };
        if current_signature == signature {
            return Ok(LocalSignatureState::Stable(signature));
        }
        signature = current_signature;
    }

    Ok(LocalSignatureState::Unstable(signature))
}

async fn read_local_signature(path: &Path) -> Result<Option<FileSignature>, String> {
    INFRASTRUCTURE
        .signature(path.to_owned())
        .await
        .map_err(|error| format!("{error:#}"))
}
