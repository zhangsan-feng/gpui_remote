use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use russh::server::Handle;
use tokio::{
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
};

/// Listeners belong to one authenticated SSH connection.
#[derive(Default)]
pub(super) struct ReverseForwards {
    listeners: HashMap<(String, u32), JoinHandle<()>>,
}

impl ReverseForwards {
    pub(super) async fn start(&mut self, address: &str, port: &mut u32, handle: Handle) -> bool {
        // Only bind the explicitly requested interface; never expand to all interfaces.
        let ip = match address.parse::<IpAddr>() {
            Ok(ip) if !ip.is_unspecified() && !ip.is_multicast() => ip,
            _ => {
                log::warn!(
                    "ssh_server_forward_rejected address={address} port={port} reason=explicit_interface_required"
                );
                return false;
            }
        };
        if *port > u16::MAX as u32 || self.listeners.contains_key(&(address.to_owned(), *port)) {
            log::warn!(
                "ssh_server_forward_rejected address={address} port={port} reason=invalid_or_duplicate"
            );
            return false;
        }
        let listener = match TcpListener::bind(SocketAddr::new(ip, *port as u16)).await {
            Ok(listener) => listener,
            Err(error) => {
                log::warn!("ssh_server_forward_bind_failed address={address} port={port}: {error}");
                return false;
            }
        };
        let local_address = match listener.local_addr() {
            Ok(address) => address,
            Err(error) => {
                log::warn!("ssh_server_forward_local_address_failed: {error}");
                return false;
            }
        };
        *port = u32::from(local_address.port());
        let key = (address.to_owned(), *port);
        let connected_address = address.to_owned();
        let connected_port = *port;
        let task = tokio::spawn(async move {
            // Aborting the listener also drops the set and cancels its relays.
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        match accepted {
                            Ok((stream, peer)) => {
                                let handle = handle.clone();
                                let address = connected_address.clone();
                                connections.spawn(relay(stream, peer, handle, address, connected_port));
                            }
                            Err(error) => {
                                log::warn!("ssh_server_forward_accept_failed listen={local_address}: {error}");
                                break;
                            }
                        }
                    }
                    result = connections.join_next(), if !connections.is_empty() => {
                        if let Some(Err(error)) = result {
                            log::warn!("ssh_server_forward_relay_task_failed: {error}");
                        }
                    }
                }
            }
        });
        self.listeners.insert(key, task);
        log::info!("ssh_server_forward_started address={address} listen={local_address}");
        true
    }

    pub(super) async fn cancel(&mut self, address: &str, port: u32) -> bool {
        let Some(task) = self.listeners.remove(&(address.to_owned(), port)) else {
            log::warn!("ssh_server_forward_cancel_missing address={address} port={port}");
            return false;
        };
        task.abort();
        let _ = task.await;
        log::info!("ssh_server_forward_cancelled address={address} port={port}");
        true
    }
}

impl Drop for ReverseForwards {
    fn drop(&mut self) {
        for ((address, port), task) in self.listeners.drain() {
            task.abort();
            log::info!(
                "ssh_server_forward_closed address={address} port={port} reason=session_closed"
            );
        }
    }
}

async fn relay(
    mut stream: TcpStream,
    peer: SocketAddr,
    handle: Handle,
    address: String,
    port: u32,
) {
    let channel = match tokio::time::timeout(
        Duration::from_secs(30),
        handle.channel_open_forwarded_tcpip(
            address.clone(),
            port,
            peer.ip().to_string(),
            u32::from(peer.port()),
        ),
    )
    .await
    {
        Ok(Ok(channel)) => channel,
        Ok(Err(error)) => {
            log::warn!(
                "ssh_server_forward_channel_failed address={address} port={port} peer={peer}: {error}"
            );
            return;
        }
        Err(_) => {
            log::warn!(
                "ssh_server_forward_channel_timeout address={address} port={port} peer={peer}"
            );
            return;
        }
    };
    let mut channel_stream = channel.into_stream();
    match tokio::io::copy_bidirectional(&mut stream, &mut channel_stream).await {
        Ok((received, sent)) => log::debug!(
            "ssh_server_forward_relay_closed address={address} port={port} peer={peer} received={received} sent={sent}"
        ),
        Err(error) => log::debug!(
            "ssh_server_forward_relay_failed address={address} port={port} peer={peer}: {error}"
        ),
    }
}
