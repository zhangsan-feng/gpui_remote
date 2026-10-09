use std::{io, net::SocketAddr};

use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;

use crate::domain::socks5_proxy::Socks5ProxyConnection;

use super::protocol::{HANDSHAKE_TIMEOUT, copy_with_idle_timeout, negotiate};

pub(super) async fn handle_client(
    mut client: TcpStream,
    connection_id: u64,
    client_address: SocketAddr,
    proxy_address: SocketAddr,
    credentials: Option<(String, String)>,
    connections: UnboundedSender<Socks5ProxyConnection>,
) -> io::Result<()> {
    let credentials = credentials
        .as_ref()
        .map(|(username, password)| (username.as_str(), password.as_str()));
    let Some(target) = tokio::time::timeout(HANDSHAKE_TIMEOUT, negotiate(&mut client, credentials))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "SOCKS5 handshake timeout"))??
    else {
        return Ok(());
    };
    let connection = Socks5ProxyConnection {
        id: connection_id,
        client: client_address,
        proxy: proxy_address,
        destination: target.peer_addr()?,
    };
    let _ = connections.send(connection);
    let (mut client_read, mut client_write) = client.into_split();
    let (mut target_read, mut target_write) = target.into_split();
    let client_to_target = copy_with_idle_timeout(&mut client_read, &mut target_write);
    let target_to_client = copy_with_idle_timeout(&mut target_read, &mut client_write);
    let (left, right) = tokio::join!(client_to_target, target_to_client);
    left.or(right).map_err(|error| {
        log::debug!("socks5_relay_closed error={error}");
        error
    })
}
