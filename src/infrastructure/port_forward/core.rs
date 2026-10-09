use std::{io::ErrorKind, net::IpAddr, time::Duration};

use tokio::{
    net::TcpListener,
    sync::oneshot,
    time::{Instant, timeout, timeout_at},
};

use crate::domain::port_forward::{PortForwardRule, PortForwardStatus};

use super::{PortForwardController, RunningForward, relay::serve};

const RULES_PATH: &str = "data/port_forward.json";
const STOP_TIMEOUT: Duration = Duration::from_secs(3);

impl PortForwardController {
    pub(super) async fn load(&self) -> Result<Vec<PortForwardRule>, String> {
        let _guard = self.persistence.lock().await;
        match tokio::fs::read(RULES_PATH).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("解析端口转发配置 {RULES_PATH} 失败: {error}")),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(format!("读取端口转发配置 {RULES_PATH} 失败: {error}")),
        }
    }

    pub(super) async fn save(&self, rules: Vec<PortForwardRule>) -> Result<(), String> {
        let _guard = self.persistence.lock().await;
        let bytes = serde_json::to_vec_pretty(&rules)
            .map_err(|error| format!("编码端口转发配置失败: {error}"))?;
        tokio::fs::create_dir_all("data")
            .await
            .map_err(|error| format!("创建端口转发配置目录失败: {error}"))?;
        let temporary = format!("{RULES_PATH}.{}.tmp", uuid::Uuid::new_v4());
        let result = async {
            tokio::fs::write(&temporary, bytes)
                .await
                .map_err(|error| format!("写入端口转发配置失败: {error}"))?;
            tokio::fs::rename(&temporary, RULES_PATH)
                .await
                .map_err(|error| format!("替换端口转发配置失败: {error}"))
        }
        .await;
        if result.is_err() {
            let _ = tokio::fs::remove_file(&temporary).await;
        }
        result
    }

    pub(super) async fn start(&self, rule: PortForwardRule) -> Result<PortForwardStatus, String> {
        let mut running = self.running.lock().await;
        if let Some(existing) = running.get(&rule.id) {
            if !existing.task.is_finished() {
                let current = self.status.borrow().get(&rule.id).cloned();
                if let Some(current) = current.filter(|status| status.running) {
                    return Ok(current);
                }
            }
        }
        if let Some(previous) = running.remove(&rule.id) {
            stop_running(previous).await;
        }
        let mut status = PortForwardStatus {
            rule_id: rule.id.clone(),
            ..Default::default()
        };
        let result = async {
            let host = rule
                .listen_host
                .parse::<IpAddr>()
                .map_err(|error| format!("监听 IP 无效: {error}"))?;
            let listener = TcpListener::bind((host, rule.listen_port))
                .await
                .map_err(|error| {
                    format!(
                        "绑定 {}:{} 失败: {error}",
                        rule.listen_host, rule.listen_port
                    )
                })?;
            let address = listener
                .local_addr()
                .map_err(|error| format!("读取监听地址失败: {error}"))?;
            Ok::<_, String>((listener, address))
        }
        .await;
        let (listener, address) = match result {
            Ok(result) => result,
            Err(error) => {
                status.error = Some(error.clone());
                self.status.send_modify(|snapshot| {
                    snapshot.insert(rule.id.clone(), status);
                });
                log::error!(
                    "port_forward_start_failed rule_id={} error={error}",
                    rule.id
                );
                return Err(error);
            }
        };
        status.running = true;
        status.listen_address = Some(address.to_string());
        self.status.send_modify(|snapshot| {
            snapshot.insert(rule.id.clone(), status.clone());
        });
        log::info!(
            "port_forward_started rule_id={} listen={} target={}:{}",
            rule.id,
            address,
            rule.target_host,
            rule.target_port
        );
        let (stop, receiver) = oneshot::channel();
        let sender = self.status.clone();
        let id = rule.id.clone();
        let task = tokio::spawn(serve(listener, rule, receiver, sender));
        running.insert(id, RunningForward { stop, task });
        Ok(status)
    }

    pub(super) async fn stop(&self, rule_id: String) -> Result<(), String> {
        let mut running = self.running.lock().await;
        if let Some(service) = running.remove(&rule_id) {
            stop_running(service).await;
        }
        self.status.send_modify(|snapshot| {
            let status = snapshot
                .entry(rule_id.clone())
                .or_insert_with(|| PortForwardStatus {
                    rule_id: rule_id.clone(),
                    ..Default::default()
                });
            status.running = false;
            status.active_connections = 0;
            status.active_connection_routes.clear();
        });
        log::info!("port_forward_stopped rule_id={rule_id} active_connections=0");
        Ok(())
    }

    pub(super) async fn shutdown(&self) -> Result<(), String> {
        let mut running = self.running.lock().await;
        // Signal every listener before waiting, so shutdown does not scale with rule count.
        let mut tasks = Vec::new();
        for (_, service) in running.drain() {
            let _ = service.stop.send(());
            tasks.push(service.task);
        }
        let deadline = Instant::now() + STOP_TIMEOUT;
        for mut task in tasks {
            if timeout_at(deadline, &mut task).await.is_err() {
                task.abort();
                let _ = task.await;
            }
        }
        self.status.send_modify(|snapshot| {
            for status in snapshot.values_mut() {
                status.running = false;
                status.active_connections = 0;
                status.active_connection_routes.clear();
            }
        });
        log::info!("port_forward_shutdown active_connections=0");
        Ok(())
    }
}

async fn stop_running(service: RunningForward) {
    let _ = service.stop.send(());
    let mut task = service.task;
    if timeout(STOP_TIMEOUT, &mut task).await.is_err() {
        task.abort();
        let _ = task.await;
    }
}
