use std::path::PathBuf;

use tokio::sync::oneshot;

use super::SftpApplication;
use crate::application::sftp::core::{SftpCommand, TransferRecord, TransferRequest, remote};

impl SftpApplication {
    pub fn sftp_transfer_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<TransferRecord>, String> {
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
            let target = remote::join_remote_path(&remote_path, &name);
            self.add_transfer(TransferRecord {
                id: transfer_id,
                name,
                direction: "上传".to_owned(),
                target: target.clone(),
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
            if runtime
                .commands
                .send(SftpCommand::Upload {
                    transfer_id,
                    local_path: path,
                    remote_path: target,
                    refresh_path: remote_path.clone(),
                    complete: None,
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
        let local_directory = self.local_directory_snapshot(workspace_id)?.path;
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
                        complete: None,
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
                let local_directory = self.local_directory_snapshot(workspace_id)?.path;
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

    pub(crate) fn upload_path_to_remote(
        &self,
        workspace_id: &str,
        local_path: PathBuf,
        remote_path: String,
    ) -> Option<oneshot::Receiver<bool>> {
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
        let (complete, completion) = oneshot::channel();
        if runtime
            .commands
            .send(SftpCommand::Upload {
                transfer_id,
                local_path,
                remote_path,
                refresh_path,
                complete: Some(complete),
            })
            .is_err()
        {
            runtime
                .model
                .set_transfer_error(transfer_id, "SFTP 连接已关闭".to_owned());
            return None;
        }
        self.inner.updates.notify_one();
        Some(completion)
    }
}
