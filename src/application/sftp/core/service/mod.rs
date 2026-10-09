mod directory;
mod transfer;

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{data_context::DATA_CONTEXT, infrastructure::INFRASTRUCTURE};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::domain::session::{Protocol, SessionProfile};

use super::{LocalSnapshot, LocalWatchRuntime, SftpCommand, SftpModel, remote};

#[derive(Clone)]
pub struct SftpApplication {
    pub(super) inner: Arc<SftpApplicationInner>,
    pub(crate) sessions: crate::application::session::SessionApplication,
}

pub(super) struct SftpApplicationInner {
    pub(super) runtimes: RwLock<HashMap<String, SftpRuntime>>,
    pub(super) local_watchers: RwLock<HashMap<String, HashMap<PathBuf, LocalWatchRuntime>>>,
}

pub(super) struct SftpRuntime {
    pub(super) model: Arc<SftpModel>,
    pub(super) commands: mpsc::UnboundedSender<SftpCommand>,
    pub(super) task: tokio::task::JoinHandle<()>,
    pub(super) navigation: Arc<Mutex<()>>,
    pub(super) local_navigation: Arc<Mutex<()>>,
}

pub(super) struct RuntimeHandles {
    pub(super) model: Arc<SftpModel>,
    pub(super) commands: mpsc::UnboundedSender<SftpCommand>,
    pub(super) navigation: Arc<Mutex<()>>,
    pub(super) local_navigation: Arc<Mutex<()>>,
}

impl SftpApplication {
    pub(crate) fn new(sessions: crate::application::session::SessionApplication) -> Self {
        Self {
            inner: Arc::new(SftpApplicationInner {
                runtimes: RwLock::new(HashMap::new()),
                local_watchers: RwLock::new(HashMap::new()),
            }),
            sessions,
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
        let local_path = match initial_local_path {
            Some(path) => path,
            None => INFRASTRUCTURE
                .default_directory()
                .await
                .map_err(|error| format!("读取默认本地目录失败: {error:#}"))?,
        };
        {
            let mut runtimes = self
                .inner
                .runtimes
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if DATA_CONTEXT.workspace_summary(&workspace_id).is_none() {
                return Err(format!("SFTP 工作区已关闭: {workspace_id}"));
            }
            if runtimes.contains_key(&workspace_id) {
                return Err(format!("SFTP 会话已打开: {workspace_id}"));
            }
            let model = Arc::new(DATA_CONTEXT.sftp_model(workspace_id.clone()));
            if !model.set_local_snapshot(LocalSnapshot {
                path: local_path,
                ..LocalSnapshot::default()
            }) {
                return Err(format!("SFTP 工作区已关闭: {workspace_id}"));
            }
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
            runtimes.insert(
                workspace_id.clone(),
                SftpRuntime {
                    model,
                    commands,
                    task,
                    navigation: Arc::new(Mutex::new(())),
                    local_navigation: Arc::new(Mutex::new(())),
                },
            );
        }
        log::debug!("SFTP application runtime registered: workspace_id={workspace_id}");
        let local_path = self.local_directory_snapshot(&workspace_id)?.path;
        let _ = self
            .update_local_directory(&workspace_id, local_path, false)
            .await;
        if DATA_CONTEXT.workspace_summary(&workspace_id).is_none() {
            return Err(format!("SFTP 工作区已关闭: {workspace_id}"));
        }
        log::debug!("SFTP application open finished: workspace_id={workspace_id}");
        Ok(())
    }

    pub async fn close(&self, workspace_id: &str) -> Result<(), String> {
        log::debug!("SFTP application close started: workspace_id={workspace_id}");
        self.close_runtime(workspace_id).await?;
        log::debug!("SFTP application close finished: workspace_id={workspace_id}");
        Ok(())
    }

    pub async fn change_remote_directory(
        &self,
        workspace_id: &str,
        path: String,
    ) -> Result<String, String> {
        log::debug!("SFTP 远程目录变更请求: workspace_id={workspace_id}, path={path}");
        let runtime = self.runtime(workspace_id)?;
        let _navigation = runtime.navigation.lock().await;
        self.ensure_workspace(workspace_id)?;
        runtime.model.set_loading();
        let (complete, result) = oneshot::channel();
        runtime
            .commands
            .send(SftpCommand::ChangeRemoteDirectory { path, complete })
            .map_err(|_| "SFTP 连接已关闭".to_owned())?;
        let actual_path = result.await.map_err(|_| "SFTP 连接已关闭".to_owned())??;
        let workspace = DATA_CONTEXT
            .workspace_summary(workspace_id)
            .ok_or_else(|| format!("SFTP 工作区已关闭: {workspace_id}"))?;
        INFRASTRUCTURE
            .update_sftp_remote_path(workspace.profile_id, actual_path.clone())
            .await
            .map_err(|error| format!("保存 SFTP 远程目录失败: {error:#}"))?;
        log::debug!("SFTP 远程目录已确认并保存: workspace_id={workspace_id}, path={actual_path}");
        Ok(actual_path)
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
                navigation: runtime.navigation.clone(),
                local_navigation: runtime.local_navigation.clone(),
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
        Ok(self.runtime(workspace_id)?.model.local_snapshot())
    }

    async fn close_runtime(&self, workspace_id: &str) -> Result<(), String> {
        let runtime = self
            .inner
            .runtimes
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id);
        self.stop_all_local_directory_listeners(workspace_id);
        let Some(runtime) = runtime else {
            return Err(format!("SFTP 会话不存在: {workspace_id}"));
        };
        let SftpRuntime { commands, task, .. } = runtime;
        let mut task = task;
        if commands.send(SftpCommand::Disconnect).is_err() {
            log::debug!("SFTP runtime command channel already closed: workspace_id={workspace_id}");
            task.abort();
            let _ = task.await;
            return Ok(());
        }
        match tokio::time::timeout(Duration::from_secs(5), &mut task).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(format!("SFTP 运行时关闭失败: {error}")),
            Err(_) => {
                log::warn!("SFTP runtime close timed out: workspace_id={workspace_id}");
                task.abort();
                let _ = task.await;
                Err(format!("SFTP 运行时关闭超时: {workspace_id}"))
            }
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
