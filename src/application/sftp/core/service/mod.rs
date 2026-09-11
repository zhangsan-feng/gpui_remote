mod directory;
mod transfer;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::{Notify, mpsc};

use crate::domain::session::{Protocol, SessionProfile};

use super::{
    LocalSnapshot, LocalWatchRuntime, SftpCommand, SftpModel, SftpSnapshot, TransferRecord,
    default_desktop_path, remote,
};

#[derive(Clone)]
pub struct SftpApplication {
    pub(super) inner: Arc<SftpApplicationInner>,
}

pub(super) struct SftpApplicationInner {
    pub(super) runtimes: RwLock<HashMap<String, SftpRuntime>>,
    pub(super) local_snapshots: RwLock<HashMap<String, LocalSnapshot>>,
    pub(super) local_watchers: RwLock<HashMap<String, HashMap<PathBuf, LocalWatchRuntime>>>,
    pub(super) transfers: Arc<RwLock<Vec<TransferRecord>>>,
    pub(super) next_transfer_id: AtomicU64,
    pub(super) updates: Arc<Notify>,
    pub(super) status_updates: Arc<Notify>,
    pub(super) transfer_ui_throttle: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
}

pub(super) struct SftpRuntime {
    pub(super) profile_ip: String,
    pub(super) profile_title: String,
    pub(super) model: Arc<SftpModel>,
    pub(super) commands: mpsc::UnboundedSender<SftpCommand>,
    pub(super) task: tokio::task::JoinHandle<()>,
}

pub(super) struct RuntimeHandles {
    pub(super) model: Arc<SftpModel>,
    pub(super) commands: mpsc::UnboundedSender<SftpCommand>,
    pub(super) profile_ip: String,
    pub(super) profile_title: String,
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
        log::debug!(
            "SFTP application open started: workspace_id={workspace_id}, profile_id={}, host={}, port={}",
            profile.id,
            profile.host,
            profile.port
        );
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
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let task_model = model.clone();
        let runtime_workspace_id = workspace_id.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = remote::run_sftp_session(
                runtime_workspace_id,
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
        log::debug!("SFTP application runtime registered: workspace_id={workspace_id}");
        self.inner
            .local_snapshots
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(workspace_id.clone())
            .or_insert_with(|| LocalSnapshot {
                path: initial_local_path.unwrap_or_else(default_desktop_path),
                ..LocalSnapshot::default()
            });
        let local_path = self.local_directory_snapshot(&workspace_id)?.path;
        let _ = self.change_local_directory(&workspace_id, local_path).await;
        self.inner.updates.notify_one();
        log::debug!("SFTP application open finished: workspace_id={workspace_id}");
        Ok(())
    }

    pub async fn close(&self, workspace_id: &str) -> Result<(), String> {
        log::debug!("SFTP application close started: workspace_id={workspace_id}");
        self.stop_all_local_directory_listeners(workspace_id);
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
        log::debug!("SFTP application close finished: workspace_id={workspace_id}");
        Ok(())
    }

    pub async fn change_remote_directory(
        &self,
        workspace_id: &str,
        path: String,
    ) -> Result<(), String> {
        log::debug!("SFTP 远程目录变更请求: workspace_id={workspace_id}, path={path}");
        let runtime = self.runtime(workspace_id)?;
        runtime.model.set_loading();
        runtime
            .commands
            .send(SftpCommand::ChangeRemoteDirectory(path))
            .map_err(|_| "SFTP 连接已关闭".to_owned())
    }

    pub fn sftp_remote_snapshot(&self, workspace_id: &str) -> Result<SftpSnapshot, String> {
        Ok(self.runtime(workspace_id)?.model.snapshot())
    }

    pub fn sftp_remote_revision(&self, workspace_id: &str) -> Result<u64, String> {
        Ok(self.runtime(workspace_id)?.model.revision())
    }

    pub fn updates(&self) -> Arc<Notify> {
        self.inner.updates.clone()
    }

    pub fn status_updates(&self) -> Arc<Notify> {
        self.inner.status_updates.clone()
    }

    pub(crate) fn runtime_available(&self, workspace_id: &str) -> bool {
        self.inner
            .runtimes
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(workspace_id)
            .is_some_and(|runtime| !runtime.commands.is_closed())
    }

    pub(super) fn runtime(&self, workspace_id: &str) -> Result<RuntimeHandles, String> {
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

    pub(super) fn ensure_workspace(&self, workspace_id: &str) -> Result<(), String> {
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

    pub(crate) fn local_directory_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<LocalSnapshot, String> {
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

    pub(super) fn next_transfer_id(&self) -> u64 {
        self.inner
            .next_transfer_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.wrapping_add(1).max(1))
            })
            .unwrap_or(1)
    }

    pub(super) fn add_transfer(&self, transfer: TransferRecord) {
        self.inner
            .transfers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(transfer);
    }

    pub(super) fn transfer_records(&self, ids: &[u64]) -> Vec<TransferRecord> {
        self.inner
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter(|transfer| ids.contains(&transfer.id))
            .cloned()
            .collect()
    }

    fn close_if_present(&self, workspace_id: &str) {
        self.stop_all_local_directory_listeners(workspace_id);
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

    fn stop_all_local_directory_listeners(&self, workspace_id: &str) {
        if let Some(watches) = self
            .inner
            .local_watchers
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
        {
            log::debug!(
                "SFTP local watchers stopping: workspace_id={workspace_id}, count={}",
                watches.len()
            );
            for watch in watches.into_values() {
                watch.stop();
            }
        }
    }
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
