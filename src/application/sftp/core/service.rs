use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::{Notify, oneshot};

use crate::domain::session::{Protocol, SessionProfile};

use super::{
    LocalSnapshot, LocalWatchRuntime, LocalWatchSummary, RemoteDeleteItem, SftpCommand, SftpModel,
    SftpRuntime, SftpSnapshot, TransferRecord, TransferRequest, default_desktop_path, remote,
};

#[derive(Clone)]
pub struct SftpApplication {
    inner: Arc<SftpApplicationInner>,
}

struct SftpApplicationInner {
    runtimes: RwLock<HashMap<String, SftpRuntime>>,
    local_snapshots: RwLock<HashMap<String, LocalSnapshot>>,
    local_watchers: RwLock<HashMap<String, HashMap<PathBuf, LocalWatchRuntime>>>,
    transfers: Arc<RwLock<Vec<TransferRecord>>>,
    next_transfer_id: AtomicU64,
    updates: Arc<Notify>,
    status_updates: Arc<Notify>,
    transfer_ui_throttle: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
}

impl Default for SftpApplication {
    fn default() -> Self {
        Self::new()
    }
}

impl SftpApplication {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(SftpApplicationInner {
                runtimes: RwLock::new(HashMap::new()),
                local_snapshots: RwLock::new(HashMap::new()),
                local_watchers: RwLock::new(HashMap::new()),
                transfers: Arc::new(RwLock::new(Vec::new())),
                next_transfer_id: AtomicU64::new(1),
                updates: Arc::new(Notify::new()),
                status_updates: Arc::new(Notify::new()),
                transfer_ui_throttle: Arc::new(std::sync::Mutex::new(None)),
            }),
        }
    }

    pub async fn open(
        &self,
        workspace_id: String,
        profile: SessionProfile,
        initial_remote_path: Option<String>,
        initial_local_path: Option<PathBuf>,
    ) -> Result<(), String> {
        if profile.protocol != Protocol::Sftp {
            return Err(format!("SFTP 模块不支持 {} 协议", profile.protocol));
        }
        self.close_if_present(&workspace_id);

        let profile_ip = profile.host.clone();
        let profile_title = profile.name.clone();

        let model = Arc::new(SftpModel::new(
            self.inner.transfers.clone(),
            self.inner.updates.clone(),
            self.inner.status_updates.clone(),
            self.inner.transfer_ui_throttle.clone(),
        ));
        let (commands, command_receiver) = tokio::sync::mpsc::unbounded_channel();
        let task_model = model.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = remote::run_sftp(
                profile,
                initial_remote_path,
                command_receiver,
                task_model.clone(),
            )
            .await
            {
                log::warn!("SFTP 运行时结束并进入失败状态: {error:#}");
                task_model.set_failed(format!("{error:#}"));
            }
        });

        self.inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(
                workspace_id.clone(),
                SftpRuntime {
                    profile_ip,
                    profile_title,
                    model,
                    commands,
                    task,
                },
            );
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(workspace_id.clone())
            .or_insert_with(|| LocalSnapshot {
                path: initial_local_path.unwrap_or_else(default_desktop_path),
                ..LocalSnapshot::default()
            });
        let local_path = self.local_snapshot(&workspace_id)?.path;
        if let Err(error) = self.change_local_directory(&workspace_id, local_path).await {
            log::debug!("读取 SFTP 本地初始目录失败: workspace_id={workspace_id}, error={error}");
        }
        self.inner.updates.notify_one();
        Ok(())
    }

    pub async fn close(&self, workspace_id: &str) -> Result<(), String> {
        self.stop_all_local_watchers(workspace_id);
        let runtime = self
            .inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))?;
        let _ = runtime.commands.send(SftpCommand::Disconnect);
        runtime.task.abort();
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id);
        self.inner.updates.notify_one();
        Ok(())
    }

    pub async fn load_directory(&self, workspace_id: &str, path: String) -> Result<(), String> {
        let runtime = self.runtime(workspace_id)?;
        runtime.model.set_loading();
        runtime
            .commands
            .send(SftpCommand::LoadDirectory(path))
            .map_err(|_| "SFTP 连接已关闭".to_owned())
    }

    pub async fn list_local(&self, workspace_id: &str) -> Result<LocalSnapshot, String> {
        self.local_snapshot(workspace_id)
    }

    pub async fn change_local_directory(
        &self,
        workspace_id: &str,
        path: PathBuf,
    ) -> Result<LocalSnapshot, String> {
        self.ensure_workspace(workspace_id)?;
        let result = tokio::task::spawn_blocking(move || super::read_local_directory(&path))
            .await
            .map_err(|error| format!("读取本地目录任务失败: {error}"))?
            .map_err(|error| format!("{error:#}"))?;
        let snapshot = LocalSnapshot {
            path: result.0,
            entries: Arc::new(result.1),
            loading: false,
            error: None,
        };
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(workspace_id.to_owned(), snapshot.clone());
        self.inner.updates.notify_one();
        Ok(snapshot)
    }

    pub async fn delete_local_paths(
        &self,
        workspace_id: &str,
        paths: Vec<PathBuf>,
    ) -> Result<LocalSnapshot, String> {
        let current_path = self.local_snapshot(workspace_id)?.path;
        let refresh_path = current_path.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut errors = Vec::new();
            for path in paths {
                if let Err(error) = super::delete_local_path(&path) {
                    log::warn!("SFTP 删除本地路径失败: {}: {error:#}", path.display());
                    errors.push(format!("{}: {error:#}", path.display()));
                }
            }
            let directory =
                super::read_local_directory(&refresh_path).map_err(|error| format!("{error:#}"))?;
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
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(workspace_id.to_owned(), snapshot.clone());
        self.inner.updates.notify_one();
        Ok(snapshot)
    }

    pub fn snapshot(&self, workspace_id: &str) -> Result<SftpSnapshot, String> {
        Ok(self.runtime(workspace_id)?.model.snapshot())
    }

    pub fn revision(&self, workspace_id: &str) -> Result<u64, String> {
        Ok(self.runtime(workspace_id)?.model.revision())
    }

    pub fn updates(&self) -> Arc<Notify> {
        self.inner.updates.clone()
    }

    pub fn status_updates(&self) -> Arc<Notify> {
        self.inner.status_updates.clone()
    }

    pub fn transfers(&self, workspace_id: &str) -> Result<Vec<TransferRecord>, String> {
        self.ensure_workspace(workspace_id)?;
        Ok(self
            .inner
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|transfer| transfer.request.workspace_id() == workspace_id)
            .cloned()
            .collect())
    }

    pub async fn watch_local(
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
        let watch = super::watcher::create(
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

    pub async fn stop_watching_local(
        &self,
        workspace_id: &str,
        local_path: &std::path::Path,
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

    pub fn local_watches(&self, workspace_id: &str) -> Result<Vec<LocalWatchSummary>, String> {
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

    pub async fn upload(
        &self,
        workspace_id: &str,
        local_paths: Vec<String>,
    ) -> Result<Vec<TransferRecord>, String> {
        if local_paths.is_empty() {
            return Err("至少需要一个本地路径".to_owned());
        }
        let runtime = self.runtime(workspace_id)?;
        let remote_path = runtime.model.snapshot().path;
        if remote_path.is_empty() {
            return Err("SFTP 尚未进入远程目录".to_owned());
        }
        let mut ids = Vec::new();
        for path in local_paths.into_iter().map(PathBuf::from) {
            let Some(name) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            let transfer_id = self.next_transfer_id();
            self.add_transfer(TransferRecord {
                id: transfer_id,
                name,
                direction: "上传".to_owned(),
                target: remote::join_remote_path(
                    &remote_path,
                    &path.file_name().unwrap().to_string_lossy(),
                ),
                request: TransferRequest::Upload {
                    workspace_id: workspace_id.to_owned(),
                    local_path: path.clone(),
                    is_directory: false,
                },
                progress: 0.,
                transferred_bytes: 0,
                total_bytes: 0,
                speed: 0,
                started_at: None,
                speed_updated_at: None,
                status: "排队中".to_owned(),
                error: None,
            });
            let target = remote::join_remote_path(
                &remote_path,
                &path.file_name().unwrap().to_string_lossy(),
            );
            if runtime
                .commands
                .send(SftpCommand::Upload {
                    transfer_id,
                    local_path: path,
                    remote_path: target,
                    refresh_path: remote_path.clone(),
                })
                .is_err()
            {
                runtime
                    .model
                    .set_transfer_error(transfer_id, "SFTP 连接已关闭".to_owned());
                continue;
            }
            ids.push(transfer_id);
        }
        if ids.is_empty() {
            return Err("没有可加入队列的本地文件或目录".to_owned());
        }
        self.inner.updates.notify_one();
        Ok(self.transfer_records(&ids))
    }

    pub async fn download(
        &self,
        workspace_id: &str,
        remote_paths: Vec<String>,
    ) -> Result<Vec<TransferRecord>, String> {
        if remote_paths.is_empty() {
            return Err("至少需要一个远程路径".to_owned());
        }
        let runtime = self.runtime(workspace_id)?;
        let local_directory = self.local_snapshot(workspace_id)?.path;
        let snapshot = runtime.model.snapshot();
        let mut ids = Vec::new();
        for remote_path in remote_paths {
            let Some(entry) = snapshot
                .entries
                .iter()
                .find(|entry| entry.path == remote_path)
            else {
                return Err(format!("当前远程目录不存在路径: {remote_path}"));
            };
            let local_path = local_directory.join(&entry.name);
            let transfer_id = self.next_transfer_id();
            self.add_transfer(TransferRecord {
                id: transfer_id,
                name: entry.name.clone(),
                direction: "下载".to_owned(),
                target: local_path.display().to_string(),
                request: TransferRequest::Download {
                    workspace_id: workspace_id.to_owned(),
                    remote_path: entry.path.clone(),
                    file_name: entry.name.clone(),
                    total_size: entry.size,
                    is_directory: entry.is_directory,
                },
                progress: 0.,
                transferred_bytes: 0,
                total_bytes: 0,
                speed: 0,
                started_at: None,
                speed_updated_at: None,
                status: "排队中".to_owned(),
                error: None,
            });
            let (complete, completion) = oneshot::channel();
            if runtime
                .commands
                .send(SftpCommand::Download {
                    transfer_id,
                    remote_path: entry.path.clone(),
                    local_path,
                    total_size: entry.size,
                    is_directory: entry.is_directory,
                    complete,
                })
                .is_err()
            {
                runtime
                    .model
                    .set_transfer_error(transfer_id, "SFTP 连接已关闭".to_owned());
                continue;
            }
            let app = self.clone();
            let workspace_id = workspace_id.to_owned();
            let local_directory = local_directory.clone();
            tokio::spawn(async move {
                if completion.await.unwrap_or(false) {
                    let _ = app
                        .change_local_directory(&workspace_id, local_directory)
                        .await;
                }
            });
            ids.push(transfer_id);
        }
        if ids.is_empty() {
            return Err("没有可加入队列的远程文件或目录".to_owned());
        }
        self.inner.updates.notify_one();
        Ok(self.transfer_records(&ids))
    }

    pub fn cancel_transfer(&self, workspace_id: &str, transfer_id: u64) -> Result<(), String> {
        let runtime = self.runtime(workspace_id)?;
        runtime.model.request_cancel(transfer_id);
        Ok(())
    }

    pub async fn retry_transfer(&self, workspace_id: &str, transfer_id: u64) -> Result<(), String> {
        let transfer = self
            .inner
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .find(|transfer| {
                transfer.id == transfer_id
                    && transfer.request.workspace_id() == workspace_id
                    && matches!(transfer.status.as_str(), "失败" | "已取消")
            })
            .cloned()
            .ok_or_else(|| format!("不存在可重试的 SFTP 传输: {transfer_id}"))?;
        let runtime = self.runtime(workspace_id)?;
        let refresh_path = runtime.model.snapshot().path;
        let new_transfer_id = self.next_transfer_id();

        match transfer.request {
            TransferRequest::Upload {
                local_path,
                is_directory,
                ..
            } => {
                self.add_transfer(TransferRecord {
                    id: new_transfer_id,
                    name: transfer.name,
                    direction: "上传".to_owned(),
                    target: transfer.target.clone(),
                    request: TransferRequest::Upload {
                        workspace_id: workspace_id.to_owned(),
                        local_path: local_path.clone(),
                        is_directory,
                    },
                    progress: 0.,
                    transferred_bytes: 0,
                    total_bytes: 0,
                    speed: 0,
                    started_at: None,
                    speed_updated_at: None,
                    status: "排队中".to_owned(),
                    error: None,
                });
                if runtime
                    .commands
                    .send(SftpCommand::Upload {
                        transfer_id: new_transfer_id,
                        local_path,
                        remote_path: transfer.target,
                        refresh_path,
                    })
                    .is_err()
                {
                    runtime
                        .model
                        .set_transfer_error(new_transfer_id, "SFTP 连接已关闭".to_owned());
                    return Err("SFTP 连接已关闭".to_owned());
                }
            }
            TransferRequest::Download {
                remote_path,
                file_name,
                total_size,
                is_directory,
                ..
            } => {
                let local_directory = self.local_snapshot(workspace_id)?.path;
                let local_path = local_directory.join(&file_name);
                self.add_transfer(TransferRecord {
                    id: new_transfer_id,
                    name: transfer.name,
                    direction: "下载".to_owned(),
                    target: local_path.display().to_string(),
                    request: TransferRequest::Download {
                        workspace_id: workspace_id.to_owned(),
                        remote_path: remote_path.clone(),
                        file_name,
                        total_size,
                        is_directory,
                    },
                    progress: 0.,
                    transferred_bytes: 0,
                    total_bytes: 0,
                    speed: 0,
                    started_at: None,
                    speed_updated_at: None,
                    status: "排队中".to_owned(),
                    error: None,
                });
                let (complete, completion) = oneshot::channel();
                if runtime
                    .commands
                    .send(SftpCommand::Download {
                        transfer_id: new_transfer_id,
                        remote_path,
                        local_path,
                        total_size,
                        is_directory,
                        complete,
                    })
                    .is_err()
                {
                    runtime
                        .model
                        .set_transfer_error(new_transfer_id, "SFTP 连接已关闭".to_owned());
                    return Err("SFTP 连接已关闭".to_owned());
                }
                let app = self.clone();
                let workspace_id = workspace_id.to_owned();
                tokio::spawn(async move {
                    if completion.await.unwrap_or(false) {
                        let _ = app
                            .change_local_directory(&workspace_id, local_directory)
                            .await;
                    }
                });
            }
        }

        self.inner.updates.notify_one();
        Ok(())
    }

    pub async fn delete_remote(
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

    fn runtime(&self, workspace_id: &str) -> Result<RuntimeHandles, String> {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .map(|runtime| RuntimeHandles {
                model: runtime.model.clone(),
                commands: runtime.commands.clone(),
                profile_ip: runtime.profile_ip.clone(),
                profile_title: runtime.profile_title.clone(),
            })
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))
    }

    fn ensure_workspace(&self, workspace_id: &str) -> Result<(), String> {
        if self
            .inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains_key(workspace_id)
        {
            Ok(())
        } else {
            Err(format!("SFTP 会话不存在: {workspace_id}"))
        }
    }

    pub(crate) fn local_snapshot(&self, workspace_id: &str) -> Result<LocalSnapshot, String> {
        self.ensure_workspace(workspace_id)?;
        Ok(self
            .inner
            .local_snapshots
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .cloned()
            .unwrap_or_default())
    }

    fn next_transfer_id(&self) -> u64 {
        self.inner
            .next_transfer_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.wrapping_add(1).max(1))
            })
            .unwrap_or(1)
    }

    fn add_transfer(&self, transfer: TransferRecord) {
        self.inner
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(transfer);
    }

    fn transfer_records(&self, ids: &[u64]) -> Vec<TransferRecord> {
        self.inner
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|transfer| ids.contains(&transfer.id))
            .cloned()
            .collect()
    }

    pub(crate) fn upload_path_to_remote(
        &self,
        workspace_id: &str,
        local_path: PathBuf,
        remote_path: String,
    ) -> Option<u64> {
        let runtime = self.runtime(workspace_id).ok()?;
        let name = local_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())?;
        let transfer_id = self.next_transfer_id();
        self.add_transfer(TransferRecord {
            id: transfer_id,
            name,
            direction: "上传".to_owned(),
            target: remote_path.clone(),
            request: TransferRequest::Upload {
                workspace_id: workspace_id.to_owned(),
                local_path: local_path.clone(),
                is_directory: false,
            },
            progress: 0.,
            transferred_bytes: 0,
            total_bytes: 0,
            speed: 0,
            started_at: None,
            speed_updated_at: None,
            status: "排队中".to_owned(),
            error: None,
        });
        let refresh_path = runtime.model.snapshot().path;
        if runtime
            .commands
            .send(SftpCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                refresh_path,
            })
            .is_err()
        {
            runtime
                .model
                .set_transfer_error(transfer_id, "SFTP 连接已关闭".to_owned());
            return None;
        }
        self.inner.updates.notify_one();
        Some(transfer_id)
    }

    fn close_if_present(&self, workspace_id: &str) {
        self.stop_all_local_watchers(workspace_id);
        if let Some(runtime) = self
            .inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
        {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }

    fn stop_all_local_watchers(&self, workspace_id: &str) {
        if let Some(watches) = self
            .inner
            .local_watchers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
        {
            for watch in watches.into_values() {
                watch.stop();
            }
        }
    }
}

struct RuntimeHandles {
    model: Arc<SftpModel>,
    commands: tokio::sync::mpsc::UnboundedSender<SftpCommand>,
    profile_ip: String,
    profile_title: String,
}

impl Drop for SftpApplicationInner {
    fn drop(&mut self) {
        let runtimes = std::mem::take(
            &mut *self
                .runtimes
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
        for runtime in runtimes.into_values() {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }
}
