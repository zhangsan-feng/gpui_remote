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
        let snapshot = self.scan_local_directory(workspace_id, path).await?;
        self.save_local_snapshot(workspace_id, snapshot.clone());
        Ok(snapshot)
    }

    async fn scan_local_directory(
        &self,
        workspace_id: &str,
        path: PathBuf,
    ) -> Result<LocalSnapshot, String> {
        self.ensure_workspace(workspace_id)?;
        let requested_path = path.display().to_string();
        log::debug!("SFTP 本地目录扫描开始: workspace_id={workspace_id}, path={requested_path}");
        let result = tokio::task::spawn_blocking(move || super::super::scan_local_directory(&path))
            .await
            .map_err(|error| format!("读取本地目录任务失败: {error}"))
            .and_then(|result| result.map_err(|error| format!("{error:#}")));
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

    fn save_local_snapshot(&self, workspace_id: &str, snapshot: LocalSnapshot) {
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(workspace_id.to_owned(), snapshot);
        self.inner.updates.notify_one();
    }

    pub async fn delete_local_paths(
        &self,
        workspace_id: &str,
        paths: Vec<PathBuf>,
    ) -> Result<LocalSnapshot, String> {
        let current_path = self.local_directory_snapshot(workspace_id)?.path;
        let refresh_path = current_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut errors = Vec::new();
            for path in paths {
                if let Err(error) = super::super::delete_local_path(&path) {
                    log::warn!("SFTP 删除本地路径失败: {}: {error:#}", path.display());
                    errors.push(format!("{}: {error:#}", path.display()));
                }
            }
            let directory = super::super::scan_local_directory(&refresh_path)
                .map_err(|error| format!("{error:#}"))?;
            Ok::<_, String>((directory, errors))
        })
        .await
        .map_err(|error| format!("删除本地路径任务失败: {error}"))??;
        let snapshot = LocalSnapshot {
            path: result.0.0,
            entries: Arc::new(result.0.1),
            loading: false,
            error: (!result.1.is_empty()).then(|| result.1.join("\n")),
        };
        self.save_local_snapshot(workspace_id, snapshot.clone());
        Ok(snapshot)
    }

    pub async fn listen_local_directory(
        &self,
        workspace_id: &str,
        local_path: PathBuf,
    ) -> Result<LocalWatchSummary, String> {
        let runtime = self.runtime(workspace_id)?;
        let remote_directory = runtime.model.snapshot().path;
        if remote_directory.is_empty() {
            return Err("SFTP 尚未进入远程目录".to_owned());
        }
        let name = local_path
            .file_name()
            .ok_or_else(|| format!("无法为本地路径生成远程目标: {}", local_path.display()))?
            .to_string_lossy();
        let remote_path = remote::join_remote_path(&remote_directory, &name);
        let watch = super::super::watcher::listen_local_directory(
            self.clone(),
            workspace_id.to_owned(),
            runtime.profile_ip,
            runtime.profile_title,
            local_path.clone(),
            remote_path,
        )
        .await?;
        let summary = watch.summary.clone();
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
        self.inner.updates.notify_one();
        Ok(summary)
    }

    pub async fn stop_listening_local_directory(
        &self,
        workspace_id: &str,
        local_path: &Path,
    ) -> Result<(), String> {
        self.ensure_workspace(workspace_id)?;
        let removed = self
            .inner
            .local_watchers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(workspace_id)
            .and_then(|watches| watches.remove(local_path));
        let Some(removed) = removed else {
            return Err(format!("本地监听不存在: {}", local_path.display()));
        };
        removed.stop();
        self.inner.updates.notify_one();
        Ok(())
    }

    pub fn sftp_local_watch_summaries(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<LocalWatchSummary>, String> {
        self.ensure_workspace(workspace_id)?;
        Ok(self
            .inner
            .local_watchers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .into_iter()
            .flat_map(|watches| watches.values())
            .map(|watch| watch.summary.clone())
            .collect())
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
