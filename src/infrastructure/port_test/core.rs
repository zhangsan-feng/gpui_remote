use std::time::Duration;

use tokio::{
    net::{TcpStream, lookup_host},
    time::timeout,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) async fn probe_tcp_port(host: String, port: u16) -> Result<(), String> {
    match timeout(PROBE_TIMEOUT, resolve_and_connect(&host, port)).await {
        Ok(result) => result,
        Err(_) => Err("TCP 端口测试超时（5 秒）".to_owned()),
    }
}

async fn resolve_and_connect(host: &str, port: u16) -> Result<(), String> {
    let addresses = lookup_host((host, port))
        .await
        .map_err(|error| format!("DNS 解析失败: {error}"))?;
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect(address).await {
            Ok(_stream) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.map_or_else(
        || "DNS 解析没有返回可用地址".to_owned(),
        |error| format!("TCP 连接失败: {error}"),
    ))
}
