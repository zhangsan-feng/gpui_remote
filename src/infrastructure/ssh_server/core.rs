use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use russh::{
    keys::{Algorithm, PrivateKey, ssh_key::LineEnding},
    server::{RunningServerHandle, Server as _},
};
use tokio::{
    net::TcpListener,
    sync::{Mutex, oneshot, watch},
    task::JoinHandle,
};

use crate::application::sftp_server::SftpServerApplication;
use crate::domain::ssh_server::{SshServerSettings, SshServerStatus};

use crate::infrastructure::sftp_server::default_root;

use super::handler::SshServer;

const SETTINGS_PATH: &str = "data/ssh_server.json";
const HOST_KEY_PATH: &str = "data/ssh_server_host_key";

struct RunningService {
    handle: RunningServerHandle,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct RuntimeState {
    running: Option<RunningService>,
}

pub(crate) struct SshServerController {
    state: Arc<Mutex<RuntimeState>>,
    generation: Arc<AtomicU64>,
    status: watch::Sender<SshServerStatus>,
}

impl SshServerController {
    pub(crate) fn new() -> Self {
        let (status, _) = watch::channel(SshServerStatus::default());
        Self {
            state: Arc::new(Mutex::new(RuntimeState::default())),
            generation: Arc::new(AtomicU64::new(0)),
            status,
        }
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<SshServerStatus> {
        self.status.subscribe()
    }

    pub(crate) async fn load_settings(&self) -> Result<SshServerSettings, String> {
        match tokio::fs::read(SETTINGS_PATH).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("读取 SSH 服务配置失败: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(SshServerSettings::default())
            }
            Err(error) => Err(format!("读取 SSH 服务配置失败: {error}")),
        }
    }

    pub(crate) async fn start(
        &self,
        settings: SshServerSettings,
        sftp_application: Arc<SftpServerApplication>,
    ) -> Result<SshServerStatus, String> {
        self.apply(settings, false, sftp_application).await
    }

    pub(crate) async fn update(
        &self,
        settings: SshServerSettings,
        sftp_application: Arc<SftpServerApplication>,
    ) -> Result<SshServerStatus, String> {
        self.apply(settings, true, sftp_application).await
    }

    async fn apply(
        &self,
        settings: SshServerSettings,
        persist: bool,
        sftp_application: Arc<SftpServerApplication>,
    ) -> Result<SshServerStatus, String> {
        let mut state = self.state.lock().await;
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(running) = state.running.take() {
            running
                .handle
                .shutdown("SSH service configuration changed".to_owned());
            running.task.abort();
        }

        let result = if settings.enabled {
            self.launch(&settings, sftp_application)
                .await
                .map(|(status, running)| {
                    state.running = Some(running);
                    status
                })
        } else {
            Ok(SshServerStatus::default())
        };
        let status = match result {
            Ok(status) => status,
            Err(error) => {
                self.status.send_replace(SshServerStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                return Err(error);
            }
        };
        if persist {
            if let Err(error) = save_settings(&settings).await {
                if let Some(running) = state.running.take() {
                    running
                        .handle
                        .shutdown("SSH service settings could not be saved".to_owned());
                    running.task.abort();
                }
                self.status.send_replace(SshServerStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                return Err(error);
            }
        }
        self.status.send_replace(status.clone());
        Ok(status)
    }

    async fn launch(
        &self,
        settings: &SshServerSettings,
        sftp_application: Arc<SftpServerApplication>,
    ) -> Result<(SshServerStatus, RunningService), String> {
        if settings.password.is_empty() {
            return Err("SSH 服务没有登录密码".to_owned());
        }
        let key = tokio::task::spawn_blocking(load_or_create_host_key)
            .await
            .map_err(|error| format!("读取 SSH 主机密钥任务失败: {error}"))??;
        let listener = TcpListener::bind((settings.host.as_str(), settings.port))
            .await
            .map_err(|error| {
                format!("SSH 监听 {}:{} 失败: {error}", settings.host, settings.port)
            })?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("读取 SSH 监听地址失败: {error}"))?;
        let root = tokio::fs::canonicalize(default_root())
            .await
            .map_err(|error| format!("准备 SFTP 根目录失败: {error}"))?;
        let config = Arc::new(russh::server::Config {
            keys: vec![key],
            inactivity_timeout: Some(Duration::from_secs(3600)),
            auth_rejection_time: Duration::from_secs(3),
            ..Default::default()
        });
        let mut server = SshServer::new(
            settings.username.clone(),
            settings.password.clone(),
            root.clone(),
            sftp_application,
        );
        let (handle_tx, handle_rx) = oneshot::channel();
        let generation = self.generation.clone();
        let current = generation.load(Ordering::SeqCst);
        let status = self.status.clone();
        let task = tokio::spawn(async move {
            let running = server.run_on_socket(config, &listener);
            let _ = handle_tx.send(running.handle());
            let result = running.await;
            if generation.load(Ordering::SeqCst) == current {
                let error = match result {
                    Ok(()) => "SSH 服务意外停止".to_owned(),
                    Err(error) => format!("SSH 服务运行失败: {error}"),
                };
                log::error!("{error}");
                status.send_replace(SshServerStatus {
                    error: Some(error),
                    ..Default::default()
                });
            }
        });
        let handle = handle_rx
            .await
            .map_err(|_| "SSH 服务启动任务中断".to_owned())?;
        log::info!(
            "ssh_server_started address={address} sftp_root={}",
            root.display()
        );
        Ok((
            SshServerStatus {
                running: true,
                address: Some(address.to_string()),
                error: None,
            },
            RunningService { handle, task },
        ))
    }

    pub(crate) async fn shutdown(&self) {
        let mut state = self.state.lock().await;
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Some(running) = state.running.take() {
            running
                .handle
                .shutdown("Application shutting down".to_owned());
            running.task.abort();
        }
        self.status.send_replace(SshServerStatus::default());
    }
}

async fn save_settings(settings: &SshServerSettings) -> Result<(), String> {
    tokio::fs::create_dir_all("data")
        .await
        .map_err(|error| format!("创建 SSH 配置目录失败: {error}"))?;
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("序列化 SSH 服务配置失败: {error}"))?;
    tokio::fs::write(SETTINGS_PATH, bytes)
        .await
        .map_err(|error| format!("保存 SSH 服务配置失败: {error}"))
}

fn load_or_create_host_key() -> Result<PrivateKey, String> {
    let path = Path::new(HOST_KEY_PATH);
    if path.exists() {
        return russh::keys::load_secret_key(path, None)
            .map_err(|error| format!("读取 SSH 主机密钥失败: {error}"));
    }
    std::fs::create_dir_all("data")
        .map_err(|error| format!("创建 SSH 主机密钥目录失败: {error}"))?;
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
        .map_err(|error| format!("生成 SSH 主机密钥失败: {error}"))?;
    let encoded = key
        .to_openssh(LineEnding::LF)
        .map_err(|error| format!("编码 SSH 主机密钥失败: {error}"))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(encoded.as_bytes())
                .map_err(|error| format!("保存 SSH 主机密钥失败: {error}"))?;
            Ok(key)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            russh::keys::load_secret_key(path, None)
                .map_err(|error| format!("读取 SSH 主机密钥失败: {error}"))
        }
        Err(error) => Err(format!("创建 SSH 主机密钥失败: {error}")),
    }
}
