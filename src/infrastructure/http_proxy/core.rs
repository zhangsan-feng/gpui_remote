use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::{
    net::TcpListener,
    sync::{mpsc::UnboundedReceiver, oneshot, watch},
    task::JoinSet,
    time::timeout,
};

use crate::domain::http_proxy::{HttpProxyConnection, HttpProxySettings, HttpProxyStatus};

use super::{ConnectionEvent, HttpProxyController, RunningService, relay::handle_client};

const SETTINGS_PATH: &str = "data/http_proxy.json";
const MAX_CONNECTIONS: usize = 128;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

impl HttpProxyController {
    pub(crate) fn subscribe_status(&self) -> watch::Receiver<HttpProxyStatus> {
        self.status.subscribe()
    }

    pub(crate) async fn load_settings(&self) -> Result<HttpProxySettings, String> {
        match tokio::fs::read(SETTINGS_PATH).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("读取 HTTP 代理配置失败: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(HttpProxySettings::default())
            }
            Err(error) => Err(format!("读取 HTTP 代理配置失败: {error}")),
        }
    }

    pub(crate) async fn start(
        &self,
        settings: HttpProxySettings,
    ) -> Result<HttpProxyStatus, String> {
        self.apply(settings, false).await
    }

    pub(crate) async fn update(
        &self,
        settings: HttpProxySettings,
    ) -> Result<HttpProxyStatus, String> {
        self.apply(settings, true).await
    }

    async fn apply(
        &self,
        settings: HttpProxySettings,
        persist: bool,
    ) -> Result<HttpProxyStatus, String> {
        let _operation = self.operation.lock().await;
        validate_settings(&settings)?;
        self.generation.fetch_add(1, Ordering::SeqCst);
        let old = self.state.lock().await.running.take();
        if let Some(running) = old {
            stop_running(running).await;
        }

        let result = if settings.enabled {
            self.launch(&settings).await
        } else {
            Ok((HttpProxyStatus::default(), None))
        };
        let (status, running) = match result {
            Ok(result) => result,
            Err(error) => {
                self.status.send_replace(HttpProxyStatus {
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
                self.status.send_replace(HttpProxyStatus {
                    error: Some(error.clone()),
                    ..Default::default()
                });
                return Err(error);
            }
        }
        self.status.send_replace(status.clone());
        if let Some(mut running) = running {
            if let Some(ready) = running.ready.take() {
                let _ = ready.send(());
            }
            self.state.lock().await.running = Some(running);
            if let Some(address) = status.address.as_ref() {
                log::info!("http_proxy_started address={address}");
            }
        }
        Ok(status)
    }

    async fn launch(
        &self,
        settings: &HttpProxySettings,
    ) -> Result<(HttpProxyStatus, Option<RunningService>), String> {
        let listener = TcpListener::bind((settings.host.as_str(), settings.port))
            .await
            .map_err(|error| {
                format!(
                    "HTTP 代理监听 {}:{} 失败: {error}",
                    settings.host, settings.port
                )
            })?;
        let address = listener
            .local_addr()
            .map_err(|error| format!("读取 HTTP 代理监听地址失败: {error}"))?;
        if !address.ip().is_loopback() && settings.username.is_empty() {
            return Err("HTTP 代理非回环监听必须配置用户名和密码".to_owned());
        }
        let credentials = if settings.username.is_empty() {
            None
        } else {
            Some((settings.username.clone(), settings.password.clone()))
        };
        let (stop, stop_receiver) = watch::channel(false);
        let (ready, ready_receiver) = oneshot::channel();
        let generation = self.generation.clone();
        let current_generation = generation.load(Ordering::SeqCst);
        let status = self.status.clone();
        let address_text = address.to_string();
        let task = tokio::spawn(async move {
            if ready_receiver.await.is_err() {
                return;
            }
            let result = run_server(
                listener,
                address_text.clone(),
                credentials,
                stop_receiver,
                status.clone(),
                generation.clone(),
                current_generation,
            )
            .await;
            if generation.load(Ordering::SeqCst) == current_generation {
                if let Err(error) = result {
                    log::error!("http_proxy_stopped error={error}");
                    status.send_replace(HttpProxyStatus {
                        error: Some(error),
                        ..Default::default()
                    });
                }
            }
        });
        Ok((
            HttpProxyStatus {
                running: true,
                address: Some(address.to_string()),
                ..Default::default()
            },
            Some(RunningService {
                stop,
                task,
                ready: Some(ready),
            }),
        ))
    }

    pub(crate) async fn shutdown(&self) {
        let _operation = self.operation.lock().await;
        self.generation.fetch_add(1, Ordering::SeqCst);
        let old = self.state.lock().await.running.take();
        if let Some(running) = old {
            stop_running(running).await;
        }
        self.status.send_replace(HttpProxyStatus::default());
    }
}

async fn run_server(
    listener: TcpListener,
    address: String,
    credentials: Option<(String, String)>,
    mut stop: watch::Receiver<bool>,
    status: watch::Sender<HttpProxyStatus>,
    generation: Arc<AtomicU64>,
    current_generation: u64,
) -> Result<(), String> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONNECTIONS));
    let active = Arc::new(AtomicUsize::new(0));
    let connection_ids = Arc::new(AtomicU64::new(1));
    let (connection_events, mut connection_event_receiver) =
        tokio::sync::mpsc::unbounded_channel::<ConnectionEvent>();
    let mut connections = Vec::new();
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            changed = stop.changed() => {
                if changed.is_err() || *stop.borrow() { break; }
            }
            joined = tasks.join_next(), if !tasks.is_empty() => {
                match joined {
                    Some(Ok(Err(error))) => log::debug!("http_proxy_client_closed error={error}"),
                    Some(Err(error)) => log::debug!("http_proxy_client_task_failed error={error}"),
                    _ => {}
                }
                let count = active.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
                drain_connection_events(&mut connection_event_receiver, &mut connections);
                publish_status(&status, &generation, current_generation, &address, count, &connections, None);
            }
            event = connection_event_receiver.recv() => {
                if let Some(event) = event {
                    apply_connection_event(&mut connections, event);
                    publish_status(
                        &status,
                        &generation,
                        current_generation,
                        &address,
                        active.load(Ordering::SeqCst),
                        &connections,
                        None,
                    );
                }
            }
            accepted = listener.accept() => {
                let (stream, peer) = accepted.map_err(|error| format!("HTTP 代理接受连接失败: {error}"))?;
                let local = stream
                    .local_addr()
                    .map_err(|error| format!("读取 HTTP 代理客户端连接地址失败: {error}"))?;
                let permit = match semaphore.clone().try_acquire_owned() {
                    Ok(permit) => permit,
                    Err(_) => {
                        log::warn!("http_proxy_connection_limit_reached peer={peer}");
                        continue;
                    }
                };
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                publish_status(&status, &generation, current_generation, &address, count, &connections, None);
                let credentials = credentials.clone();
                let connection_events = connection_events.clone();
                let connection_ids = connection_ids.clone();
                let mut child_stop = stop.clone();
                tasks.spawn(async move {
                    let result = tokio::select! {
                        result = handle_client(stream, peer, local, credentials, connection_events, connection_ids) => result,
                        _ = child_stop.changed() => Ok(()),
                    };
                    drop(permit);
                    result
                });
            }
        }
    }
    while let Some(joined) = tasks.join_next().await {
        if let Err(error) = joined {
            log::debug!("http_proxy_client_shutdown_failed error={error}");
        }
        let count = active.fetch_sub(1, Ordering::SeqCst).saturating_sub(1);
        drain_connection_events(&mut connection_event_receiver, &mut connections);
        publish_status(
            &status,
            &generation,
            current_generation,
            &address,
            count,
            &connections,
            None,
        );
    }
    Ok(())
}

fn publish_status(
    status: &watch::Sender<HttpProxyStatus>,
    generation: &AtomicU64,
    current_generation: u64,
    address: &str,
    active_connections: usize,
    connections: &[HttpProxyConnection],
    error: Option<String>,
) {
    if generation.load(Ordering::SeqCst) != current_generation {
        return;
    }
    status.send_replace(HttpProxyStatus {
        running: true,
        address: Some(address.to_owned()),
        active_connections,
        connections: connections.to_vec(),
        error,
    });
}

fn drain_connection_events(
    events: &mut UnboundedReceiver<ConnectionEvent>,
    connections: &mut Vec<HttpProxyConnection>,
) {
    while let Ok(event) = events.try_recv() {
        apply_connection_event(connections, event);
    }
}

fn apply_connection_event(connections: &mut Vec<HttpProxyConnection>, event: ConnectionEvent) {
    match event {
        ConnectionEvent::Opened(connection) => {
            if let Some(existing) = connections
                .iter_mut()
                .find(|existing| existing.id == connection.id)
            {
                *existing = connection;
            } else {
                connections.push(connection);
            }
        }
        ConnectionEvent::Closed(id) => connections.retain(|connection| connection.id != id),
    }
}

async fn stop_running(mut running: RunningService) {
    let _ = running.stop.send(true);
    running.ready.take();
    let mut task = running.task;
    if timeout(SHUTDOWN_TIMEOUT, &mut task).await.is_err() {
        task.abort();
        let _ = task.await;
    }
}

async fn save_settings(settings: &HttpProxySettings) -> Result<(), String> {
    tokio::fs::create_dir_all("data")
        .await
        .map_err(|error| format!("创建 HTTP 代理配置目录失败: {error}"))?;
    let bytes = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("序列化 HTTP 代理配置失败: {error}"))?;
    tokio::fs::write(SETTINGS_PATH, bytes)
        .await
        .map_err(|error| format!("保存 HTTP 代理配置失败: {error}"))
}

fn validate_settings(settings: &HttpProxySettings) -> Result<(), String> {
    if settings.host.trim().is_empty() || settings.port == 0 {
        return Err("HTTP 代理监听地址不能为空，端口必须大于 0".to_owned());
    }
    if settings.username.is_empty() != settings.password.is_empty() {
        return Err("HTTP 代理用户名和密码必须同时配置".to_owned());
    }
    if settings.username.contains(':') {
        return Err("HTTP Basic 用户名不能包含冒号".to_owned());
    }
    Ok(())
}
