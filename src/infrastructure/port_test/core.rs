use std::{process::Stdio, time::Duration};

use tokio::{
    net::{TcpStream, lookup_host},
    process::Command,
    time::timeout,
};

use super::PortTestProbeResult;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const ICMP_PROBE_TIMEOUT: Duration = Duration::from_secs(4);

pub(super) async fn probe_tcp_port(host: String, port: u16) -> PortTestProbeResult {
    let tcp_result = match timeout(PROBE_TIMEOUT, resolve_and_connect(&host, port)).await {
        Ok(result) => result,
        Err(_) => Err("TCP 端口测试超时（5 秒）".to_owned()),
    };

    match tcp_result {
        Ok(()) => PortTestProbeResult {
            reachable: true,
            host_reachable: None,
            error: None,
        },
        Err(tcp_error) => match ping_host(&host).await {
            Ok(host_reachable) => PortTestProbeResult {
                reachable: false,
                host_reachable: Some(host_reachable),
                error: Some(if host_reachable {
                    tcp_error
                } else {
                    format!("{tcp_error}；主机 {host} ICMP 不可达")
                }),
            },
            Err(icmp_error) => PortTestProbeResult {
                reachable: false,
                host_reachable: None,
                error: Some(format!("{tcp_error}；{icmp_error}")),
            },
        },
    }
}

async fn ping_host(host: &str) -> Result<bool, String> {
    let mut command = Command::new("ping");
    #[cfg(target_os = "windows")]
    command.args(["-n", "1", "-w", "3000"]);
    #[cfg(target_os = "macos")]
    command.args(["-c", "1", "-W", "3000"]);
    #[cfg(target_os = "linux")]
    command.args(["-c", "1", "-W", "3"]);
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    command.args(["-c", "1"]);

    command
        .arg(host)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    match timeout(ICMP_PROBE_TIMEOUT, command.status()).await {
        Ok(Ok(status)) => Ok(status.success()),
        Ok(Err(error)) => Err(format!("ICMP 主机探测启动失败: {error}")),
        Err(_) => Ok(false),
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
