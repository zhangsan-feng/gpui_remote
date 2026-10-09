use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{
    net::TcpListener,
    sync::{Mutex, mpsc::UnboundedReceiver, watch},
    task::{JoinHandle, JoinSet},
    time::timeout,
};

use crate::domain::socks5_proxy::{Socks5ProxyConnection, Socks5ProxySettings, Socks5ProxyStatus};

use super::{protocol::reject_busy, relay::handle_client};

const SETTINGS_PATH: &str = "data/socks5_proxy.json";
const MAX_CONNECTIONS: usize = 128;
const MAX_REJECTED_CONNECTIONS: usize = 128;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

struct RunningService {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct RuntimeState {
    running: Option<RunningService>,
}

pub(crate) struct Socks5ProxyController {
    operation: Arc<Mutex<()>>,
    state: Arc<Mutex<RuntimeState>>,
    generation: Arc<AtomicU64>,
    status: watch::Sender<Socks5ProxyStatus>,
}

impl Socks5ProxyController {
    pub(crate) fn new() -> Self {
        let (status, _) = watch::channel(Socks5ProxyStatus::default());
        Self {
            operation: Arc::new(Mutex::new(())),
            state: Arc::new(Mutex::new(RuntimeState::default())),
            generation: Arc::new(AtomicU64::new(0)),
            status,
        }
    }

    pub(crate) fn subscribe_status(&self) -> watch::Receiver<Socks5ProxyStatus> {
        self.status.subscribe()
    }

    pub(crate) async fn load_settings(&self) -> Result<Socks5ProxySettings, String> {
        match tokio::fs::read(SETTINGS_PATH).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("读取 SOCKS5 配置失败: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Socks5ProxySettings::default())
            }
            Err(error) => Err(format!("读取 SOCKS5 配置失败: {error}")),
        }
    }

    pub(crate) async fn start(
        &self,
        settings: Socks5ProxySettings,
    ) -> Result<Socks5ProxyStatus, String> {
        self.apply(settings, false).await
    }

    pub(crate) async fn update(
        &self,
        settings: Socks5ProxySettings,
    ) -> Result<Socks5ProxyStatus, String> {
        self.apply(settings, true).await
    }

    async fn apply(
        &self,
        settings: Socks5ProxySettings,
        persist: bool,
    ) -> Result<Socks5ProxyStatus, String> {
        let _operation = self.operation.lock().await;
        self.generation.fetch_add(1, Ordering::SeqCst);
        let old = self.state.lock().await.running.take();
        if let Some(running) = old {
            stop_running(running).await;
        }

        let result = if settings.enabled {
            self.launch(&settings).await
        } else {
            Ok((Socks5ProxyStatus::default(), None))
        };
        let (status, running) = match result {
            Ok(result) => result,
            Err(error) => {
                self.status.send_replace(Socks5ProxyStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                return Err(error);
            }
        };
        if persist {
            if let Err(error) = save_settings(&settings).await {
                if let Some(running) = running {
                    stop_running(running).await;
                }
                self.status.send_replace(Socks5ProxyStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                return Err(error);
            }
        }
        if let Some(running) = running {
            self.state.lock().await.running = Some(running);
        }
        self.status.send_replace(status.clone());
        Ok(status)
    }

    async fn launch(
        &self,
        settings: &Socks5ProxySettings,
    ) -> Result<(Socks5ProxyStatus, Option<RunningService>), String> {
        let listener = TcpListener::bind((settings.host.as_str(), settings.port))
            .await
            .map_err(|error| {
                format!(
                    "SOCKS5 监听 {}:{} 失败: {error}",
                    settings.host, settings.port
                )
            })?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("读取 SOCKS5 监听地址失败: {error}"))?;
        let credentials = if settings.username.is_empty() {
            None
        } else {
            Some((settings.username.clone(), settings.password.clone()))
        };
        let (stop, stop_receiver) = watch::channel(false);
        let generation = self.generation.clone();
        let current_generation = generation.load(Ordering::SeqCst);
        let status = self.status.clone();
        let task = tokio::spawn(async move {
            let result = run_server(
                listener,
                address,
                credentials,
                stop_receiver,
                status.clone(),
                generation.clone(),
                current_generation,
            )
            .await;
            if generation.load(Ordering::SeqCst) == current_generation {
                if let Err(error) = result {
                    log::error!("socks5_proxy_stopped error={error}");
                    status.send_replace(Socks5ProxyStatus {
                        error: Some(error),
                        ..Default::default()
                    });
                }
            }
        });
        log::info!("socks5_proxy_started address={address}");
        Ok((
            Socks5ProxyStatus {
                running: true,
                address: Some(address.to_string()),
                ..Default::default()
            },
            Some(RunningService { stop, task }),
        ))
    }

    pub(crate) async fn shutdown(&self) {
        let _operation = self.operation.lock().await;
        self.generation.fetch_add(1, Ordering::SeqCst);
        let old = self.state.lock().await.running.take();
        if let Some(running) = old {
            stop_running(running).await;
        }
        self.status.send_replace(Socks5ProxyStatus::default());
    }
}

async fn run_server(
    listener: TcpListener,
    address: SocketAddr,
    credentials: Option<(String, String)>,
    mut stop: watch::Receiver<bool>,
    status: watch::Sender<Socks5ProxyStatus>,
    generation: Arc<AtomicU64>,
    current_generation: u64,
) -> Result<(), String> {
    let address_text = address.to_string();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    let rejected_semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_REJECTED_CONNECTIONS));
    let active = Arc::new(AtomicUsize::new(0));
    let (connection_events, mut connection_event_receiver) =
        tokio::sync::mpsc::unbounded_channel::<Socks5ProxyConnection>();
    let mut connections = Vec::new();
    let mut next_connection_id = 1_u64;
    let mut tasks = JoinSet::new();
    let mut rejected_tasks = JoinSet::new();
    loop {
        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() { break; }
            }
            joined = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(joined) = joined {
                    let completed_connection_id = match joined {
                        Ok((connection_id, client_address, result)) => {
                            if let Err(error) = result {
                                log::debug!("socks5_client_task_failed peer={client_address} error={error}");
                            }
                            Some(connection_id)
                        }
                        Err(error) => {
                            log::debug!("socks5_client_task_failed error={error}");
                            None
                        }
                    };
                    let count = active.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
                    drain_connection_events(&mut connection_event_receiver, &mut connections);
                    if let Some(connection_id) = completed_connection_id {
                        connections.retain(|connection| connection.id != connection_id);
                    }
                    publish_status(&status, &generation, current_generation, &address_text, count, &connections, None);
                }
            }
            connection = connection_event_receiver.recv() => {
                if let Some(connection) = connection {
                    upsert_connection(&mut connections, connection);
                    publish_status(
                        &status,
                        &generation,
                        current_generation,
                        &address_text,
                        active.load(Ordering::SeqCst),
                        &connections,
                        None,
                    );
                }
            }
            joined = rejected_tasks.join_next(), if !rejected_tasks.is_empty() => {
                if let Some(Err(error)) = joined {
                    log::debug!("socks5_busy_task_failed error={error}");
                }
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted.map_err(|error| format!("SOCKS5 接受连接失败: {error}"))?;
                let permit = match semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        log::warn!("socks5_connection_limit_reached peer={peer}");
                        let Ok(permit) = rejected_semaphore.clone().try_acquire_owned() else {
                            log::warn!("socks5_rejected_connection_limit_reached peer={peer}");
                            continue;
                        };
                        rejected_tasks.spawn(async move {
                            let _permit = permit;
                            reject_busy(stream).await
                        });
                        continue;
                    }
                };
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                publish_status(&status, &generation, current_generation, &address_text, count, &connections, None);
                let connection_id = next_connection_id;
                next_connection_id = next_connection_id.wrapping_add(1);
                let credentials = credentials.clone();
                let connection_events = connection_events.clone();
                let mut child_stop = stop.clone();
                tasks.spawn(async move {
                    let result = tokio::select! {
                        result = handle_client(stream, connection_id, peer, address, credentials, connection_events) => result,
                        _ = child_stop.changed() => Ok(()),
                    };
                    drop(permit);
                    (connection_id, peer, result)
                });
            }
        }
    }
    while let Some(joined) = tasks.join_next().await {
        let completed_connection_id = match joined {
            Ok((connection_id, client_address, result)) => {
                if let Err(error) = result {
                    log::debug!(
                        "socks5_client_shutdown_failed peer={client_address} error={error}"
                    );
                }
                Some(connection_id)
            }
            Err(error) => {
                log::debug!("socks5_client_shutdown_failed error={error}");
                None
            }
        };
        let count = active.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
        drain_connection_events(&mut connection_event_receiver, &mut connections);
        if let Some(connection_id) = completed_connection_id {
            connections.retain(|connection| connection.id != connection_id);
        }
        publish_status(
            &status,
            &generation,
            current_generation,
            &address_text,
            count,
            &connections,
            None,
        );
    }
    while let Some(joined) = rejected_tasks.join_next().await {
        if let Err(error) = joined {
            log::debug!("socks5_busy_shutdown_failed error={error}");
        }
    }
    Ok(())
}

fn publish_status(
    status: &watch::Sender<Socks5ProxyStatus>,
    generation: &AtomicU64,
    current_generation: u64,
    address: &str,
    active_connections: usize,
    connections: &[Socks5ProxyConnection],
    error: Option<String>,
) {
    if generation.load(Ordering::SeqCst) != current_generation {
        return;
    }
    status.send_replace(Socks5ProxyStatus {
        running: true,
        address: Some(address.to_owned()),
        active_connections,
        connections: connections.to_vec(),
        error,
    });
}

fn drain_connection_events(
    events: &mut UnboundedReceiver<Socks5ProxyConnection>,
    connections: &mut Vec<Socks5ProxyConnection>,
) {
    while let Ok(connection) = events.try_recv() {
        upsert_connection(connections, connection);
    }
}

fn upsert_connection(
    connections: &mut Vec<Socks5ProxyConnection>,
    connection: Socks5ProxyConnection,
) {
    if let Some(existing) = connections
        .iter_mut()
        .find(|existing| existing.id == connection.id)
    {
        *existing = connection;
    } else {
        connections.push(connection);
    }
}

async fn stop_running(running: RunningService) {
    let _ = running.stop.send(true);
    let mut task = running.task;
    if timeout(SHUTDOWN_TIMEOUT, &mut task).await.is_err() {
        task.abort();
        let _ = task.await;
    }
}

async fn save_settings(settings: &Socks5ProxySettings) -> Result<(), String> {
    tokio::fs::create_dir_all("data")
        .await
        .map_err(|error| format!("创建 SOCKS5 配置目录失败: {error}"))?;
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("序列化 SOCKS5 配置失败: {error}"))?;
    tokio::fs::write(SETTINGS_PATH, bytes)
        .await
        .map_err(|error| format!("保存 SOCKS5 配置失败: {error}"))
}
