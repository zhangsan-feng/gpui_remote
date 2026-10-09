use std::{path::Path, sync::Arc};

use super::{conn, transfer};

use anyhow::{Context as _, Result, bail};
use russh::{Disconnect, client};
use russh_sftp::client::SftpSession;

use crate::{
    application::ports::{
        RemoteDeleteItem, SftpCancellationCallback, SftpConnectFuture, SftpConnection, SftpEntry,
        SftpFuture, SftpProgressCallback, SftpTransferFuture,
    },
    domain::session::SessionProfile,
};

use crate::infrastructure::connection::ConnectionPort;

pub(super) fn connect<'a>(
    connection: &'a dyn ConnectionPort,
    workspace_id: &'a str,
    profile: &'a SessionProfile,
) -> SftpConnectFuture<'a> {
    Box::pin(async move {
        log::debug!(
            "SFTP infrastructure connecting: workspace_id={workspace_id}, host={}, port={}, proxy_enabled={}",
            profile.host,
            profile.port,
            profile.proxy.is_some()
        );
        let stream = connection
            .connect(profile.host.as_str(), profile.port, profile.proxy.as_ref())
            .await?;
        log::debug!("SFTP TCP transport connected: workspace_id={workspace_id}");
        let mut session = client::connect_stream(
            Arc::new(conn::ssh_config()),
            stream,
            conn::SftpClientHandler {
                endpoint: format!("[{}]:{}", profile.host, profile.port),
            },
        )
        .await
        .context("SFTP SSH 握手或主机密钥校验失败")?;
        log::debug!("SFTP SSH handshake completed: workspace_id={workspace_id}");
        let authentication = if let Some(path) = profile.private_key_path.as_deref() {
            let key_path = path.to_owned();
            let key = tokio::task::spawn_blocking({
                let key_path = key_path.clone();
                move || russh::keys::load_secret_key(&key_path, None)
            })
            .await
            .context("加载 SFTP SSH 私钥任务失败")?
            .with_context(|| format!("加载 SFTP SSH 私钥失败: {key_path}"))?;
            session
                .authenticate_publickey(
                    profile.username.clone(),
                    russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await
                .context("SFTP SSH 私钥认证失败")?
        } else {
            session
                .authenticate_password(profile.username.clone(), profile.password.clone())
                .await
                .context("SFTP SSH 密码认证失败")?
        };
        if !authentication.success() {
            if profile.private_key_path.is_some() {
                bail!("SFTP 用户名或私钥错误");
            }
            bail!("SFTP 用户名或密码错误");
        }
        log::debug!("SFTP SSH authentication succeeded: workspace_id={workspace_id}");

        let channel = session
            .channel_open_session()
            .await
            .context("创建 SFTP 会话通道失败")?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .context("启动远程 SFTP 子系统失败")?;
        let sftp = Arc::new(
            SftpSession::new(channel.into_stream())
                .await
                .context("初始化 SFTP 协议失败")?,
        );
        log::debug!(
            "SFTP infrastructure connected: workspace_id={workspace_id}, host={}, port={}",
            profile.host,
            profile.port
        );
        Ok(Box::new(RusshSftpConnection { sftp, session }) as Box<dyn SftpConnection>)
    })
}

struct RusshSftpConnection {
    sftp: Arc<SftpSession>,
    session: client::Handle<conn::SftpClientHandler>,
}

impl SftpConnection for RusshSftpConnection {
    fn canonicalize<'a>(&'a self, path: &'a str) -> SftpFuture<'a, String> {
        Box::pin(async move {
            self.sftp
                .canonicalize(path)
                .await
                .with_context(|| format!("解析远程路径 {path} 失败"))
        })
    }

    fn read_directory<'a>(&'a self, path: &'a str) -> SftpFuture<'a, Vec<SftpEntry>> {
        Box::pin(async move { scan_remote_directory(&self.sftp, path).await })
    }

    fn delete_paths<'a>(&'a self, items: &'a [RemoteDeleteItem]) -> SftpFuture<'a, ()> {
        Box::pin(async move { delete_remote_paths(&self.sftp, items).await })
    }

    fn upload_path<'a>(
        &'a self,
        local_path: &'a Path,
        remote_path: &'a str,
        mut on_progress: SftpProgressCallback<'a>,
        is_cancelled: SftpCancellationCallback<'a>,
    ) -> SftpTransferFuture<'a> {
        Box::pin(async move {
            transfer::upload_path(
                &self.sftp,
                local_path,
                remote_path,
                move |transferred, total| on_progress(transferred, total),
                move || is_cancelled(),
            )
            .await
        })
    }

    fn download_path<'a>(
        &'a self,
        remote_path: &'a str,
        local_path: &'a Path,
        is_directory: bool,
        mut on_progress: SftpProgressCallback<'a>,
        is_cancelled: SftpCancellationCallback<'a>,
    ) -> SftpTransferFuture<'a> {
        Box::pin(async move {
            transfer::download_path(
                &self.sftp,
                remote_path,
                local_path,
                0,
                is_directory,
                move |transferred, total| on_progress(transferred, total),
                move || is_cancelled(),
            )
            .await
        })
    }

    fn close(&self) -> SftpFuture<'_, ()> {
        Box::pin(async move {
            let close_sftp = self.sftp.close().await.context("关闭 SFTP 客户端失败");
            let disconnect = self
                .session
                .disconnect(Disconnect::ByApplication, "", "zh-CN")
                .await
                .context("关闭 SFTP SSH transport 失败");
            close_sftp.and(disconnect)
        })
    }
}

pub(super) fn join_remote_path(directory: &str, file_name: &str) -> String {
    if directory == "/" {
        format!("/{file_name}")
    } else {
        format!("{}/{file_name}", directory.trim_end_matches('/'))
    }
}

async fn scan_remote_directory(sftp: &SftpSession, path: &str) -> Result<Vec<SftpEntry>> {
    let mut entries = sftp
        .read_dir(path)
        .await
        .with_context(|| format!("读取远程目录 {path} 失败"))?
        .map(|entry| {
            let metadata = entry.metadata();
            SftpEntry {
                name: entry.file_name(),
                path: entry.path(),
                is_directory: metadata.is_dir(),
                size: metadata.len(),
                modified_at: metadata.mtime,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

async fn delete_remote_path(
    sftp: &SftpSession,
    remote_path: &str,
    is_directory: bool,
) -> Result<()> {
    if !is_directory {
        return sftp
            .remove_file(remote_path)
            .await
            .with_context(|| format!("删除远程文件 {remote_path} 失败"));
    }
    let mut directories = Vec::new();
    let mut pending = vec![remote_path.to_owned()];
    while let Some(directory) = pending.pop() {
        directories.push(directory.clone());
        let children = sftp
            .read_dir(&directory)
            .await
            .with_context(|| format!("读取远程目录 {directory} 失败"))?;
        for child in children {
            let name = child.file_name();
            if name == "." || name == ".." {
                continue;
            }
            if child.metadata().is_dir() {
                pending.push(child.path());
            } else {
                let child_path = child.path();
                sftp.remove_file(&child_path)
                    .await
                    .with_context(|| format!("删除远程文件 {child_path} 失败"))?;
            }
        }
    }
    for directory in directories.into_iter().rev() {
        sftp.remove_dir(&directory)
            .await
            .with_context(|| format!("删除远程目录 {directory} 失败"))?;
    }
    Ok(())
}

async fn delete_remote_paths(sftp: &SftpSession, items: &[RemoteDeleteItem]) -> Result<()> {
    log::debug!("SFTP 批量删除远程路径开始: count={}", items.len());
    let mut errors = Vec::new();
    for item in items {
        log::debug!(
            "SFTP 删除远程路径: path={}, is_directory={}",
            item.path,
            item.is_directory
        );
        if let Err(error) = delete_remote_path(sftp, &item.path, item.is_directory).await {
            let error =
                anyhow::anyhow!(error).context(format!("批量删除远程路径 {} 失败", item.path));
            log::warn!("{error:#}");
            errors.push(format!("{error:#}"));
        }
    }
    if !errors.is_empty() {
        bail!("{}", errors.join("\n"));
    }
    log::debug!("SFTP 批量删除远程路径完成: count={}", items.len());
    Ok(())
}
