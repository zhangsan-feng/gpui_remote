use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use gpui_kit::*;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, oneshot};

use super::super::{SftpView, StopWatchingLocalPath, WatchLocalPath};
use crate::application::agent_mcp::SftpWatchSummary;

const WATCH_DEBOUNCE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalWatchKind {
    File,
    Directory,
}

pub(crate) struct LocalWatch {
    kind: LocalWatchKind,
    remote_path: String,
    stop: Option<oneshot::Sender<()>>,
}

impl LocalWatch {
    fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn is_local_path_watched(
        &self,
        workspace_id: &str,
        local_path: &Path,
    ) -> bool {
        self.local_watchers
            .get(workspace_id)
            .is_some_and(|watches| watches.contains_key(local_path))
    }

    pub(in crate::gui::workspace::sftp) fn local_watched_paths(&self) -> HashSet<PathBuf> {
        self.selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.local_watchers.get(workspace_id))
            .map(|watches| watches.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(in crate::gui::workspace::sftp) fn watch_local_path(
        &mut self,
        action: &WatchLocalPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        match self.watch_local_path_for_workspace(&workspace_id, action.0.clone(), cx) {
            Ok(completion) => {
                cx.spawn(async move |_this, _cx| match completion.await {
                    Ok(Err(error)) => log::warn!("开启本地自动上传监听失败: {error}"),
                    Err(_) => log::warn!("开启本地自动上传监听任务已取消"),
                    Ok(Ok(_)) => {}
                })
                .detach();
            }
            Err(error) => {
                log::warn!("开启本地自动上传监听失败: {error}");
            }
        }
    }

    pub(in crate::gui::workspace::sftp) fn watch_local_path_for_workspace(
        &mut self,
        workspace_id: &str,
        local_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<oneshot::Receiver<Result<SftpWatchSummary, String>>, String> {
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return Err(format!("SFTP 会话不存在: {workspace_id}"));
        };
        let remote_directory = runtime.model.snapshot().path;
        let profile_ip = runtime.profile_ip.clone();
        let profile_title = runtime.profile_title.clone();
        if remote_directory.is_empty() {
            return Err("SFTP 尚未进入远程目录".to_owned());
        }

        let Some(remote_path) = remote_path_for_watch(&local_path, &remote_directory) else {
            return Err(format!(
                "无法为本地路径生成远程目标: {}",
                local_path.display()
            ));
        };
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel::<notify::Result<Event>>();
        let (stop_sender, mut stop_receiver) = oneshot::channel();
        let local_root = local_path.clone();
        let remote_root = remote_path.clone();
        let task_workspace_id = workspace_id.to_owned();
        let task_local_root = local_root.clone();
        let task_remote_root = remote_root.clone();
        let summary_local_path = local_root.display().to_string();
        let summary_remote_path = remote_root.clone();
        let (completion_sender, completion_receiver) = oneshot::channel();
        let setup_local_root = local_root.clone();
        let setup_workspace_id = task_workspace_id.clone();
        cx.spawn(async move |this, cx| {
            let setup = tokio::task::spawn_blocking(move || {
                let metadata = fs::metadata(&setup_local_root).map_err(|error| {
                    format!(
                        "监听本地路径不存在: {}: {error}",
                        setup_local_root.display()
                    )
                })?;
                let kind = if metadata.is_dir() {
                    LocalWatchKind::Directory
                } else if metadata.is_file() {
                    LocalWatchKind::File
                } else {
                    return Err(format!(
                        "不支持监听本地路径类型: {}",
                        setup_local_root.display()
                    ));
                };
                let watch_root = watch_root(&setup_local_root, kind);
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
            .map_err(|error| format!("创建本地监听任务失败: {error}"))
            .and_then(|result| result);

            let (watcher, kind) = match setup {
                Ok(result) => result,
                Err(error) => {
                    let _ = completion_sender.send(Err(error));
                    return;
                }
            };
            let install_result = this.update(cx, |this, cx| {
                if !this.runtimes.contains_key(&setup_workspace_id) {
                    return Err(format!("SFTP 会话不存在: {setup_workspace_id}"));
                }
                this.stop_local_watch_for_workspace(&setup_workspace_id, &local_root);
                this.local_watchers
                    .entry(setup_workspace_id.clone())
                    .or_default()
                    .insert(
                        local_root.clone(),
                        LocalWatch {
                            kind,
                            remote_path: summary_remote_path.clone(),
                            stop: Some(stop_sender),
                        },
                    );
                cx.notify();
                Ok(())
            });
            match install_result {
                Ok(Ok(())) => {
                    let _ = completion_sender.send(Ok(SftpWatchSummary {
                        workspace_id: task_workspace_id.clone(),
                        ip: profile_ip,
                        title: profile_title,
                        local_path: summary_local_path,
                        remote_path: summary_remote_path,
                        is_directory: kind == LocalWatchKind::Directory,
                        debounce_ms: WATCH_DEBOUNCE.as_millis() as u64,
                    }));
                }
                Ok(Err(error)) => {
                    let _ = completion_sender.send(Err(error));
                    return;
                }
                Err(_) => {
                    let _ = completion_sender.send(Err("工作区已关闭".to_owned()));
                    return;
                }
            }

            let _watcher = watcher;
            let mut pending_paths = HashSet::new();
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
                            let Some(event) = event else {
                                return;
                            };
                            collect_event_paths(
                                event,
                                kind,
                                &task_local_root,
                                &mut pending_paths,
                            );
                        }
                        _ = &mut sleep => break,
                    }
                }

                let paths = std::mem::take(&mut pending_paths);
                if this
                    .update(cx, |this, cx| {
                        this.upload_watched_paths(
                            &task_workspace_id,
                            &task_local_root,
                            &task_remote_root,
                            kind,
                            paths,
                            cx,
                        );
                    })
                    .is_err()
                {
                    return;
                }
            }
        })
        .detach();
        Ok(completion_receiver)
    }

    pub(in crate::gui::workspace::sftp) fn stop_watching_local_path(
        &mut self,
        action: &StopWatchingLocalPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        self.stop_local_watch_for_workspace(&workspace_id, &action.0);
        cx.notify();
    }

    pub(in crate::gui::workspace::sftp) fn stop_watching_local_path_for_workspace(
        &mut self,
        workspace_id: &str,
        local_path: &Path,
    ) -> Result<(), String> {
        if self.stop_local_watch_for_workspace(workspace_id, local_path) {
            Ok(())
        } else {
            Err(format!("本地监听不存在: {}", local_path.display()))
        }
    }

    pub(in crate::gui::workspace::sftp) fn local_watch_summaries(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<SftpWatchSummary>, String> {
        let runtime = self
            .runtimes
            .get(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        Ok(self
            .local_watchers
            .get(workspace_id)
            .into_iter()
            .flat_map(|watches| watches.iter())
            .map(|(local_path, watch)| SftpWatchSummary {
                workspace_id: workspace_id.to_owned(),
                ip: runtime.profile_ip.clone(),
                title: runtime.profile_title.clone(),
                local_path: local_path.display().to_string(),
                remote_path: watch.remote_path.clone(),
                is_directory: watch.kind == LocalWatchKind::Directory,
                debounce_ms: WATCH_DEBOUNCE.as_millis() as u64,
            })
            .collect())
    }

    pub(in crate::gui::workspace::sftp) fn stop_local_watchers_for_workspace(
        &mut self,
        workspace_id: &str,
    ) {
        if let Some(watches) = self.local_watchers.remove(workspace_id) {
            for watch in watches.into_values() {
                watch.stop();
            }
        }
    }

    pub(in crate::gui::workspace::sftp) fn stop_all_local_watchers(&mut self) {
        for watches in self.local_watchers.drain().map(|(_, watches)| watches) {
            for watch in watches.into_values() {
                watch.stop();
            }
        }
    }

    fn stop_local_watch_for_workspace(&mut self, workspace_id: &str, local_path: &Path) -> bool {
        let Some(watches) = self.local_watchers.get_mut(workspace_id) else {
            return false;
        };
        let removed = if let Some(watch) = watches.remove(local_path) {
            watch.stop();
            true
        } else {
            false
        };
        if watches.is_empty() {
            self.local_watchers.remove(workspace_id);
        }
        removed
    }

    fn upload_watched_paths(
        &mut self,
        workspace_id: &str,
        local_root: &Path,
        remote_root: &str,
        kind: LocalWatchKind,
        paths: HashSet<PathBuf>,
        cx: &mut Context<Self>,
    ) {
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
            let remote_path = match kind {
                LocalWatchKind::File => remote_root.to_owned(),
                LocalWatchKind::Directory => {
                    let relative_path = path
                        .strip_prefix(local_root)
                        .map(|path| path.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    if relative_path.is_empty() {
                        remote_root.to_owned()
                    } else {
                        super::remote::join_remote_path(remote_root, &relative_path)
                    }
                }
            };
            if self
                .upload_file_to_remote_path(workspace_id, path.clone(), remote_path, cx)
                .is_some()
                && kind == LocalWatchKind::Directory
            {
                uploaded_roots.push(path);
            }
        }
    }
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

fn remote_path_for_watch(local_path: &Path, remote_directory: &str) -> Option<String> {
    let name = local_path.file_name()?.to_string_lossy();
    Some(super::remote::join_remote_path(remote_directory, &name))
}

fn collect_event_paths(
    result: notify::Result<Event>,
    kind: LocalWatchKind,
    local_root: &Path,
    pending_paths: &mut HashSet<PathBuf>,
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
