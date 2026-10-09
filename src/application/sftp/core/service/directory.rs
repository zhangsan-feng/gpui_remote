use crate::{data_context::DATA_CONTEXT, infrastructure::INFRASTRUCTURE};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use super::SftpApplication;
use crate::application::sftp::core::{
    LocalSnapshot, LocalWatchSummary, RemoteDeleteItem, SftpCommand, remote,
};

impl SftpApplication {
    pub async fn change_local_directory(
        &self,
        workspace_id: &str,
        path: PathBuf,
    ) -> Result<LocalSnapshot, String> {
        let runtime = self.runtime(workspace_id)?;
        let _navigation = runtime.local_navigation.lock().await;
        let snapshot = self
            .update_local_directory(workspace_id, path, false)
            .await?
            .ok_or_else(|| format!("SFTP 本地目录请求已过期: {workspace_id}"))?;
        let workspace = DATA_CONTEXT
            .workspace_summary(workspace_id)
            .ok_or_else(|| format!("SFTP 工作区已关闭: {workspace_id}"))?;
        INFRASTRUCTURE
            .update_sftp_local_path(workspace.profile_id, snapshot.path.clone())
            .await
            .map_err(|error| format!("保存 SFTP 本地目录失败: {error:#}"))?;
        log::debug!(
            "SFTP 本地目录已确认并保存: workspace_id={workspace_id}, path={}",
            snapshot.path.display()
        );
        Ok(snapshot)
    }

    pub(super) async fn refresh_local_directory_if_current(
        &self,
        workspace_id: &str,
        path: PathBuf,
    ) -> Result<(), String> {
        self.update_local_directory(workspace_id, path, true)
            .await
            .map(|_| ())
    }

    pub(super) async fn update_local_directory(
        &self,
        workspace_id: &str,
        path: PathBuf,
        only_if_current: bool,
    ) -> Result<Option<LocalSnapshot>, String> {
        let model = self.runtime(workspace_id)?.model;
        let Some(generation) = model.begin_local_scan(&path, only_if_current) else {
            return Ok(None);
        };
        let snapshot = match self.scan_local_directory(workspace_id, path).await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                model.fail_local_scan(generation, error.clone());
                return Err(error);
            }
        };
        if self.save_local_snapshot(workspace_id, generation, snapshot.clone())? {
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    async fn scan_local_directory(
        &self,
        workspace_id: &str,
        path: PathBuf,
    ) -> Result<LocalSnapshot, String> {
        self.ensure_workspace(workspace_id)?;
        let requested_path = path.display().to_string();
        log::debug!("SFTP 本地目录扫描开始: workspace_id={workspace_id}, path={requested_path}");
        let result = INFRASTRUCTURE
            .scan_directory(path)
            .await
            .map_err(|error| format!("{error:#}"));
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                log::debug!(
                    "SFTP 本地目录扫描失败: workspace_id={workspace_id}, path={requested_path}, error={error}"
                );
                return Err(error);
            }
        };
        let snapshot = LocalSnapshot {
            path: result.0,
            entries: Arc::new(result.1),
            loading: false,
            error: None,
        };
        log::debug!(
            "SFTP 本地目录扫描完成: workspace_id={workspace_id}, path={}, entries={}",
            snapshot.path.display(),
            snapshot.entries.len()
        );
        Ok(snapshot)
    }

    fn save_local_snapshot(
        &self,
        workspace_id: &str,
        generation: u64,
        snapshot: LocalSnapshot,
    ) -> Result<bool, String> {
        let runtimes = self
            .inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(runtime) = runtimes.get(workspace_id) else {
            log::debug!("SFTP stale local scan ignored: workspace_id={workspace_id}");
            return Err(format!("SFTP 会话已关闭: {workspace_id}"));
        };
        let committed = runtime.model.finish_local_scan(generation, snapshot);
        if !committed {
            log::debug!(
                "SFTP stale local scan result ignored: workspace_id={workspace_id}, generation={generation}"
            );
        }
        drop(runtimes);
        Ok(committed)
    }

    pub async fn delete_local_paths(
        &self,
        workspace_id: &str,
        paths: Vec<PathBuf>,
    ) -> Result<LocalSnapshot, String> {
        let current_path = self.local_directory_snapshot(workspace_id)?.path;
        let model = self.runtime(workspace_id)?.model;
        let generation = model
            .begin_local_scan(&current_path, true)
            .ok_or_else(|| format!("SFTP 本地目录已变化: {workspace_id}"))?;
        let refresh_path = current_path.clone();
        let mut errors = Vec::new();
        for path in paths {
            if let Err(error) = INFRASTRUCTURE.delete_path(path.clone()).await {
                log::warn!("SFTP 删除本地路径失败: {}: {error:#}", path.display());
                errors.push(format!("{}: {error:#}", path.display()));
            }
        }
        let directory = INFRASTRUCTURE
            .scan_directory(refresh_path)
            .await
            .map_err(|error| format!("{error:#}"));
        let directory = match directory {
            Ok(directory) => directory,
            Err(error) => {
                model.fail_local_scan(generation, error.clone());
                return Err(error);
            }
        };
        let snapshot = LocalSnapshot {
            path: directory.0,
            entries: Arc::new(directory.1),
            loading: false,
            error: (!errors.is_empty()).then(|| errors.join("\n")),
        };
        self.save_local_snapshot(workspace_id, generation, snapshot.clone())?;
        Ok(snapshot)
    }

    pub async fn listen_local_directory(
        &self,
        workspace_id: &str,
        local_path: PathBuf,
    ) -> Result<LocalWatchSummary, String> {
        let runtime = self.runtime(workspace_id)?;
        let workspace = DATA_CONTEXT
            .workspace_summary(workspace_id)
            .filter(|workspace| workspace.protocol == crate::domain::session::Protocol::Sftp)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let remote_directory = runtime.model.snapshot().path;
        if remote_directory.is_empty() {
            return Err("SFTP 尚未进入远程目录".to_owned());
        }
        let name = local_path
            .file_name()
            .ok_or_else(|| format!("无法为本地路径生成远程目标: {}", local_path.display()))?
            .to_string_lossy();
        let remote_path = remote::join_remote_path(&remote_directory, &name);
        let (watch, summary) = super::super::watcher::listen_local_directory(
            self.clone(),
            workspace_id.to_owned(),
            workspace.host,
            workspace.title,
            local_path.clone(),
            remote_path,
        )
        .await?;
        // Keep the runtime read lock through registration. Closing removes the
        // runtime first, then drains watchers, so a late setup cannot reinsert one.
        let runtimes = self
            .inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !runtimes.contains_key(workspace_id) {
            watch.stop();
            return Err(format!("SFTP 会话已关闭: {workspace_id}"));
        }
        if !runtime.model.add_watch(summary.clone()) {
            watch.stop();
            return Err(format!("SFTP 工作区数据已关闭: {workspace_id}"));
        }
        if let Some(previous) = self
            .inner
            .local_watchers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(workspace_id.to_owned())
            .or_default()
            .insert(local_path, watch)
        {
            previous.stop();
        }
        drop(runtimes);
        Ok(summary)
    }

    pub async fn stop_listening_local_directory(
        &self,
        workspace_id: &str,
        local_path: &Path,
    ) -> Result<(), String> {
        self.ensure_workspace(workspace_id)?;
        let runtime = self.runtime(workspace_id)?;
        let removed = {
            let mut watchers = self
                .inner
                .local_watchers
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            watchers
                .get_mut(workspace_id)
                .and_then(|watches| watches.remove(local_path))
        };
        let Some(removed) = removed else {
            return Err(format!("本地监听不存在: {}", local_path.display()));
        };
        removed.stop();
        runtime
            .model
            .remove_watch(&local_path.display().to_string());
        Ok(())
    }

    pub async fn delete_remote_paths(
        &self,
        workspace_id: &str,
        items: Vec<RemoteDeleteItem>,
    ) -> Result<(), String> {
        let runtime = self.runtime(workspace_id)?;
        let refresh_path = runtime.model.snapshot().path;
        runtime
            .commands
            .send(SftpCommand::Delete {
                items,
                refresh_path,
            })
            .map_err(|_| "SFTP 连接已关闭".to_owned())
    }
}
