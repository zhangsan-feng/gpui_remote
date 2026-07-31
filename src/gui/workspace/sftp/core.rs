use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{Context as _, Result, bail};
use gpui::*;
use russh::{Disconnect, client};
use russh_sftp::client::SftpSession;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::mpsc,
};
use tokio_socks::tcp::Socks5Stream;

use crate::{domain::session::SessionProfile, infrastructure::storage::verify_host_key};

use super::{
    LocalEntry, SftpCommand, SftpEntry, SftpModel, SftpRuntime, SftpSnapshot, SftpStatus, SftpView,
};

trait SshStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> SshStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

struct SftpClientHandler {
    endpoint: String,
}

impl client::Handler for SftpClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match verify_host_key(&self.endpoint, server_public_key) {
            Ok(accepted) => Ok(accepted),
            Err(error) => {
                log::info!("SFTP host key verification failed: {error:#}");
                Ok(false)
            }
        }
    }
}

impl SftpModel {
    pub(super) fn snapshot(&self) -> SftpSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn update(&self, update: impl FnOnce(&mut SftpSnapshot), status_changed: bool) {
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            update(&mut snapshot);
        }
        self.updates.notify_waiters();
        if status_changed {
            self.status_updates.notify_waiters();
        }
    }

    fn set_connected(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(
            |snapshot| {
                snapshot.status = SftpStatus::Connected;
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            true,
        );
    }

    fn set_loading(&self) {
        self.update(
            |snapshot| {
                snapshot.loading = true;
                snapshot.error = None;
            },
            false,
        );
    }

    fn set_directory(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(
            |snapshot| {
                snapshot.path = path;
                snapshot.entries = Arc::new(entries);
                snapshot.loading = false;
                snapshot.error = None;
            },
            false,
        );
    }

    fn set_error(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            false,
        );
    }

    fn set_failed(&self, error: String) {
        self.update(
            |snapshot| {
                snapshot.status = SftpStatus::Failed;
                snapshot.loading = false;
                snapshot.error = Some(error);
            },
            true,
        );
    }
}

impl SftpView {
    pub(super) fn load_local_directory(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.local.path = path.clone();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || read_local_directory(&path))
                .await
                .map_err(|error| anyhow::anyhow!("读取本地目录任务失败: {error}"))
                .and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                this.local.loading = false;
                match result {
                    Ok((path, entries)) => {
                        this.local.path = path;
                        this.local.entries = Arc::new(entries);
                        this.local.error = None;
                    }
                    Err(error) => {
                        this.local.error = Some(format!("{error:#}"));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn connect(&mut self, workspace_id: String, profile: SessionProfile) {
        self.close(&workspace_id);
        self.remote_list_state.reset_with_uniform_height(0, px(38.));

        let model = Arc::new(SftpModel {
            snapshot: RwLock::new(SftpSnapshot::default()),
            updates: self.updates.clone(),
            status_updates: self.status_updates.clone(),
        });
        let (commands, command_receiver) = mpsc::unbounded_channel();
        let task_model = model.clone();
        let task = tokio::spawn(async move {
            if let Err(error) = run_sftp(profile, command_receiver, task_model.clone()).await {
                task_model.set_failed(format!("{error:#}"));
            }
        });
        self.runtimes.insert(
            workspace_id,
            SftpRuntime {
                model,
                commands,
                task,
            },
        );
    }

    pub(super) fn close(&mut self, workspace_id: &str) {
        if let Some(runtime) = self.runtimes.remove(workspace_id) {
            let _ = runtime.commands.send(SftpCommand::Disconnect);
            runtime.task.abort();
        }
    }

    pub(super) fn load_directory(&self, path: String) {
        let Some(runtime) = self
            .selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.runtimes.get(workspace_id))
        else {
            return;
        };
        self.remote_list_state.reset_with_uniform_height(0, px(38.));
        runtime.model.set_loading();
        if runtime
            .commands
            .send(SftpCommand::LoadDirectory(path))
            .is_err()
        {
            runtime.model.set_error("SFTP 连接已关闭".to_owned());
        }
    }
}

pub(super) fn default_desktop_path() -> PathBuf {
    let profile = std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let desktop = profile.join("Desktop");
    if desktop.is_dir() { desktop } else { profile }
}

fn read_local_directory(path: &Path) -> Result<(PathBuf, Vec<LocalEntry>)> {
    let path = path
        .canonicalize()
        .with_context(|| format!("无法访问本地目录 {}", path.display()))?;
    let mut entries = fs::read_dir(&path)
        .with_context(|| format!("读取本地目录 {} 失败", path.display()))?
        .filter_map(|entry| match entry {
            Ok(entry) => {
                let metadata = match entry.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        log::debug!("读取本地文件信息失败: {error}");
                        return None;
                    }
                };
                Some(LocalEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path(),
                    is_directory: metadata.is_dir(),
                    size: metadata.len(),
                    modified_at: metadata.modified().ok(),
                })
            }
            Err(error) => {
                log::debug!("读取本地目录项失败: {error}");
                None
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .is_directory
            .cmp(&left.is_directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok((path, entries))
}

async fn run_sftp(
    profile: SessionProfile,
    mut commands: mpsc::UnboundedReceiver<SftpCommand>,
    model: Arc<SftpModel>,
) -> Result<()> {
    let stream = connect_transport(&profile).await?;
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(30)),
        keepalive_interval: Some(Duration::from_secs(15)),
        keepalive_max: 3,
        ..Default::default()
    });
    let mut session = client::connect_stream(
        config,
        stream,
        SftpClientHandler {
            endpoint: format!("[{}]:{}", profile.host, profile.port),
        },
    )
    .await
    .context("SFTP SSH 握手或主机密钥校验失败")?;
    let authentication = session
        .authenticate_password(profile.username.clone(), profile.password.clone())
        .await
        .context("SFTP SSH 密码认证失败")?;
    if !authentication.success() {
        bail!("SFTP 用户名或密码错误");
    }

    let channel = session
        .channel_open_session()
        .await
        .context("创建 SFTP 会话通道失败")?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .context("启动远程 SFTP 子系统失败")?;
    let sftp = SftpSession::new(channel.into_stream())
        .await
        .context("初始化 SFTP 协议失败")?;

    let initial_path = sftp
        .canonicalize(".")
        .await
        .context("读取 SFTP 初始目录失败")?;
    let entries = read_directory(&sftp, &initial_path).await?;
    model.set_connected(initial_path, entries);

    while let Some(command) = commands.recv().await {
        match command {
            SftpCommand::LoadDirectory(path) => {
                let result = async {
                    let path = sftp.canonicalize(path).await.context("解析远程目录失败")?;
                    let entries = read_directory(&sftp, &path).await?;
                    Ok::<_, anyhow::Error>((path, entries))
                }
                .await;
                match result {
                    Ok((path, entries)) => model.set_directory(path, entries),
                    Err(error) => model.set_error(format!("{error:#}")),
                }
            }
            SftpCommand::Disconnect => break,
        }
    }

    let _ = sftp.close().await;
    let _ = session
        .disconnect(Disconnect::ByApplication, "", "zh-CN")
        .await;
    model.update(
        |snapshot| {
            snapshot.status = SftpStatus::Disconnected;
            snapshot.loading = false;
        },
        true,
    );
    Ok(())
}

async fn read_directory(sftp: &SftpSession, path: &str) -> Result<Vec<SftpEntry>> {
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

async fn connect_transport(profile: &SessionProfile) -> Result<Box<dyn SshStream>> {
    let target = (profile.host.as_str(), profile.port);
    if let Some(proxy) = &profile.proxy {
        let proxy_address = (proxy.host.as_str(), proxy.port);
        let stream = if proxy.username.is_empty() {
            Socks5Stream::connect(proxy_address, target).await
        } else {
            Socks5Stream::connect_with_password(
                proxy_address,
                target,
                &proxy.username,
                &proxy.password,
            )
            .await
        }
        .with_context(|| format!("连接 SOCKS5 代理 {}:{} 失败", proxy.host, proxy.port))?;
        Ok(Box::new(stream))
    } else {
        let stream = TcpStream::connect(target)
            .await
            .with_context(|| format!("连接 SFTP 主机 {}:{} 失败", profile.host, profile.port))?;
        Ok(Box::new(stream))
    }
}
