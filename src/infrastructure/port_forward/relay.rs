use std::time::Duration;

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{oneshot, watch},
    task::JoinSet,
    time::timeout,
};

use crate::domain::port_forward::{PortForwardConnection, PortForwardRule};

use super::StatusSnapshot;

const MAX_CONNECTIONS: usize = 128;
const DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) async fn serve(
    listener: TcpListener,
    rule: PortForwardRule,
    mut stop: oneshot::Receiver<()>,
    status: watch::Sender<StatusSnapshot>,
) {
    let mut clients: JoinSet<Result<(), String>> = JoinSet::new();
    let mut active_routes: Vec<(tokio::task::Id, PortForwardConnection)> = Vec::new();
    let mut recent_error = None;
    loop {
        tokio::select! {
            biased;
            _ = &mut stop => break,
            completed = clients.join_next_with_id(), if !clients.is_empty() => {
                let error = match completed {
                    Some(Ok((task_id, result))) => {
                        if let Some(index) = active_routes
                            .iter()
                            .position(|(id, _)| *id == task_id)
                        {
                            active_routes.remove(index);
                        }
                        result.err()
                    }
                    Some(Err(error)) => {
                        let task_id = error.id();
                        if let Some(index) = active_routes
                            .iter()
                            .position(|(id, _)| *id == task_id)
                        {
                            active_routes.remove(index);
                        }
                        Some(format!("转发任务失败: {error}"))
                    }
                    _ => None,
                };
                if let Some(error) = error {
                    log::warn!("port_forward_client_failed rule_id={} listen={}:{} target={}:{} active_connections={} error={error}", rule.id, rule.listen_host, rule.listen_port, rule.target_host, rule.target_port, clients.len());
                    recent_error = Some(error);
                }
                update(
                    &status,
                    &rule.id,
                    true,
                    clients.len(),
                    active_routes
                        .iter()
                        .map(|(_, route)| route.clone())
                        .collect(),
                    recent_error.clone(),
                );
            }
            accepted = listener.accept() => {
                match accepted {
                    Ok((client, peer)) => {
                        if clients.len() >= MAX_CONNECTIONS {
                            log::warn!("port_forward_connection_limit rule_id={} peer={} active_connections={}", rule.id, peer, clients.len());
                            drop(client);
                            continue;
                        }
                        let route = PortForwardConnection {
                            client_address: peer.to_string(),
                            target_address: endpoint(&rule.target_host, rule.target_port),
                        };
                        let task = clients.spawn(relay(client, rule.target_host.clone(), rule.target_port));
                        active_routes.insert(0, (task.id(), route));
                        update(
                            &status,
                            &rule.id,
                            true,
                            clients.len(),
                            active_routes
                                .iter()
                                .map(|(_, route)| route.clone())
                                .collect(),
                            recent_error.clone(),
                        );
                        log::debug!("port_forward_client_accepted rule_id={} peer={} active_connections={}", rule.id, peer, clients.len());
                    }
                    Err(error) => {
                        let error = format!("监听 {}:{} 接受连接失败: {error}", rule.listen_host, rule.listen_port);
                        log::error!("port_forward_listener_failed rule_id={} error={error}", rule.id);
                        recent_error = Some(error);
                        break;
                    }
                }
            }
        }
    }
    // Release the port immediately, then give existing clients a bounded drain period.
    drop(listener);
    update(
        &status,
        &rule.id,
        false,
        clients.len(),
        active_routes
            .iter()
            .map(|(_, route)| route.clone())
            .collect(),
        recent_error.clone(),
    );
    if timeout(DRAIN_TIMEOUT, async {
        while clients.join_next().await.is_some() {}
    })
    .await
    .is_err()
    {
        clients.abort_all();
        while clients.join_next().await.is_some() {}
    }
    update(&status, &rule.id, false, 0, Vec::new(), recent_error);
    log::info!(
        "port_forward_listener_stopped rule_id={} active_connections=0",
        rule.id
    );
}

async fn relay(mut client: TcpStream, host: String, port: u16) -> Result<(), String> {
    let mut target = timeout(CONNECT_TIMEOUT, TcpStream::connect((host.as_str(), port)))
        .await
        .map_err(|_| format!("连接目标 {host}:{port} 超时"))?
        .map_err(|error| format!("连接目标 {host}:{port} 失败: {error}"))?;
    tokio::io::copy_bidirectional(&mut client, &mut target)
        .await
        .map_err(|error| format!("向目标 {host}:{port} 转发失败: {error}"))?;
    Ok(())
}

fn update(
    status: &watch::Sender<StatusSnapshot>,
    id: &str,
    running: bool,
    count: usize,
    routes: Vec<PortForwardConnection>,
    error: Option<String>,
) {
    status.send_modify(|snapshot| {
        if let Some(current) = snapshot.get_mut(id) {
            current.running = running;
            current.active_connections = count;
            current.active_connection_routes = routes;
            current.error = error;
        }
    });
}

fn endpoint(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
