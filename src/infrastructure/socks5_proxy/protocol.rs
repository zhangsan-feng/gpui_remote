use std::{
    io,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    time::{Duration, timeout},
};

const SOCKS_VERSION: u8 = 5;
const NO_AUTH: u8 = 0x00;
const USERNAME_PASSWORD: u8 = 0x02;
const NO_ACCEPTABLE_METHOD: u8 = 0xff;
const AUTH_VERSION: u8 = 1;
const AUTH_SUCCESS: u8 = 0x00;
const AUTH_FAILURE: u8 = 0x01;
const CONNECT: u8 = 0x01;
const REQUEST_SUCCEEDED: u8 = 0x00;
const GENERAL_FAILURE: u8 = 0x01;
const NETWORK_UNREACHABLE: u8 = 0x03;
const HOST_UNREACHABLE: u8 = 0x04;
const CONNECTION_REFUSED: u8 = 0x05;
const TTL_EXPIRED: u8 = 0x06;
const COMMAND_NOT_SUPPORTED: u8 = 0x07;
const ADDRESS_TYPE_NOT_SUPPORTED: u8 = 0x08;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(crate) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn negotiate(
    stream: &mut TcpStream,
    credentials: Option<(&str, &str)>,
) -> io::Result<Option<TcpStream>> {
    let methods = read_greeting(stream).await?;
    let selected = match credentials {
        Some(_) if methods.contains(&USERNAME_PASSWORD) => USERNAME_PASSWORD,
        None if methods.contains(&NO_AUTH) => NO_AUTH,
        _ => {
            stream
                .write_all(&[SOCKS_VERSION, NO_ACCEPTABLE_METHOD])
                .await?;
            return Ok(None);
        }
    };
    stream.write_all(&[SOCKS_VERSION, selected]).await?;
    if selected == USERNAME_PASSWORD
        && !authenticate(stream, credentials.expect("credentials selected")).await?
    {
        return Ok(None);
    }

    let (command, target) = read_request(stream).await?;
    if command != CONNECT {
        write_response(stream, COMMAND_NOT_SUPPORTED, None).await?;
        return Ok(None);
    }
    let target = match connect_target(&target).await {
        Ok(target) => target,
        Err(code) => {
            write_response(stream, code, None).await?;
            return Ok(None);
        }
    };
    let bound = target.local_addr().ok();
    write_response(stream, REQUEST_SUCCEEDED, bound).await?;
    Ok(Some(target))
}

pub(crate) async fn reject_busy(mut stream: TcpStream) -> io::Result<()> {
    let methods = timeout(CONNECT_TIMEOUT, read_greeting(&mut stream)).await;
    let Ok(Ok(methods)) = methods else {
        return Ok(());
    };
    let selected = if methods.contains(&NO_AUTH) {
        NO_AUTH
    } else if methods.contains(&USERNAME_PASSWORD) {
        USERNAME_PASSWORD
    } else {
        NO_ACCEPTABLE_METHOD
    };
    stream.write_all(&[SOCKS_VERSION, selected]).await?;
    if selected == USERNAME_PASSWORD {
        let mut auth_header = [0u8; 2];
        timeout(CONNECT_TIMEOUT, stream.read_exact(&mut auth_header)).await??;
        if auth_header[0] != AUTH_VERSION {
            return Ok(());
        }
        let _ = timeout(CONNECT_TIMEOUT, read_string(&mut stream, auth_header[1])).await??;
        let mut password_length = [0u8; 1];
        timeout(CONNECT_TIMEOUT, stream.read_exact(&mut password_length)).await??;
        let _ = timeout(
            CONNECT_TIMEOUT,
            read_string(&mut stream, password_length[0]),
        )
        .await??;
        stream.write_all(&[AUTH_VERSION, AUTH_FAILURE]).await?;
        return Ok(());
    }
    if selected == NO_ACCEPTABLE_METHOD {
        return Ok(());
    }
    let mut request = [0u8; 4];
    if timeout(CONNECT_TIMEOUT, stream.read_exact(&mut request))
        .await
        .is_err()
    {
        return Ok(());
    }
    stream
        .write_all(&[SOCKS_VERSION, GENERAL_FAILURE, 0, 1, 0, 0, 0, 0, 0, 0])
        .await
}

async fn read_greeting(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;
    if header[0] != SOCKS_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid SOCKS version",
        ));
    }
    let mut methods = vec![0u8; header[1] as usize];
    stream.read_exact(&mut methods).await?;
    Ok(methods)
}

async fn authenticate(stream: &mut TcpStream, credentials: (&str, &str)) -> io::Result<bool> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).await?;
    if header[0] != AUTH_VERSION {
        stream.write_all(&[AUTH_VERSION, AUTH_FAILURE]).await?;
        return Ok(false);
    }
    let username = read_string(stream, header[1]).await?;
    let mut length = [0u8; 1];
    stream.read_exact(&mut length).await?;
    let password = read_string(stream, length[0]).await?;
    let valid = username == credentials.0.as_bytes() && password == credentials.1.as_bytes();
    stream
        .write_all(&[
            AUTH_VERSION,
            if valid { AUTH_SUCCESS } else { AUTH_FAILURE },
        ])
        .await?;
    Ok(valid)
}

async fn read_string(stream: &mut TcpStream, length: u8) -> io::Result<Vec<u8>> {
    let mut value = vec![0u8; length as usize];
    stream.read_exact(&mut value).await?;
    Ok(value)
}

async fn read_request(stream: &mut TcpStream) -> io::Result<(u8, TargetAddress)> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != SOCKS_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid SOCKS request version",
        ));
    }
    let target = match header[3] {
        0x01 => {
            let mut bytes = [0u8; 4];
            stream.read_exact(&mut bytes).await?;
            TargetAddress::Ip(IpAddr::V4(Ipv4Addr::from(bytes)), read_port(stream).await?)
        }
        0x03 => {
            let mut length = [0u8; 1];
            stream.read_exact(&mut length).await?;
            if length[0] == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "empty SOCKS domain",
                ));
            }
            let domain = read_string(stream, length[0]).await?;
            TargetAddress::Domain(
                String::from_utf8_lossy(&domain).into_owned(),
                read_port(stream).await?,
            )
        }
        0x04 => {
            let mut bytes = [0u8; 16];
            stream.read_exact(&mut bytes).await?;
            TargetAddress::Ip(IpAddr::V6(Ipv6Addr::from(bytes)), read_port(stream).await?)
        }
        _ => {
            return Ok((header[1], TargetAddress::Unsupported));
        }
    };
    Ok((header[1], target))
}

async fn read_port(stream: &mut TcpStream) -> io::Result<u16> {
    let mut bytes = [0u8; 2];
    stream.read_exact(&mut bytes).await?;
    Ok(u16::from_be_bytes(bytes))
}

async fn connect_target(target: &TargetAddress) -> Result<TcpStream, u8> {
    match target {
        TargetAddress::Ip(ip, port) => timeout(CONNECT_TIMEOUT, TcpStream::connect((*ip, *port)))
            .await
            .map_err(|_| TTL_EXPIRED)?
            .map_err(map_connect_error),
        TargetAddress::Domain(domain, port) => {
            let addresses = timeout(CONNECT_TIMEOUT, lookup_host((domain.as_str(), *port)))
                .await
                .map_err(|_| TTL_EXPIRED)?
                .map_err(|_| HOST_UNREACHABLE)?
                .collect::<Vec<_>>();
            if addresses.is_empty() {
                return Err(HOST_UNREACHABLE);
            }
            let mut last_error = GENERAL_FAILURE;
            for address in addresses {
                match timeout(CONNECT_TIMEOUT, TcpStream::connect(address)).await {
                    Ok(Ok(stream)) => return Ok(stream),
                    Ok(Err(error)) => last_error = map_connect_error(error),
                    Err(_) => last_error = TTL_EXPIRED,
                }
            }
            Err(last_error)
        }
        TargetAddress::Unsupported => Err(ADDRESS_TYPE_NOT_SUPPORTED),
    }
}

fn map_connect_error(error: io::Error) -> u8 {
    match error.kind() {
        io::ErrorKind::ConnectionRefused => CONNECTION_REFUSED,
        io::ErrorKind::NetworkUnreachable => NETWORK_UNREACHABLE,
        io::ErrorKind::AddrNotAvailable | io::ErrorKind::NotFound => HOST_UNREACHABLE,
        io::ErrorKind::TimedOut => TTL_EXPIRED,
        _ => GENERAL_FAILURE,
    }
}

async fn write_response(
    stream: &mut TcpStream,
    code: u8,
    bound: Option<SocketAddr>,
) -> io::Result<()> {
    let bound = bound.unwrap_or_else(|| SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0));
    let mut response = Vec::with_capacity(22);
    response.extend_from_slice(&[SOCKS_VERSION, code, 0x00]);
    match bound.ip() {
        IpAddr::V4(ip) => {
            response.push(0x01);
            response.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            response.push(0x04);
            response.extend_from_slice(&ip.octets());
        }
    }
    response.extend_from_slice(&bound.port().to_be_bytes());
    stream.write_all(&response).await
}

#[derive(Debug)]
enum TargetAddress {
    Ip(IpAddr, u16),
    Domain(String, u16),
    Unsupported,
}

pub(crate) async fn copy_with_idle_timeout<R, W>(reader: &mut R, writer: &mut W) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let read = timeout(Duration::from_secs(300), reader.read(&mut buffer))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "SOCKS5 relay idle timeout"))??;
        if read == 0 {
            writer.shutdown().await?;
            return Ok(());
        }
        writer.write_all(&buffer[..read]).await?;
    }
}
