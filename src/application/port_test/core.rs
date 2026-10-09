use std::time::Instant;

use crate::{
    application::{ApplicationResult, port_test::PortTestResult},
    infrastructure::INFRASTRUCTURE,
};

const PORT_ERROR: &str = "端口必须是 1 到 65535 之间的十进制整数";

pub(super) async fn test_tcp_port(host: String, port: String) -> ApplicationResult<PortTestResult> {
    let host = host.trim().to_owned();
    if host.is_empty() {
        return Err("主机名不能为空".to_owned());
    }

    let port = port.trim();
    if port.is_empty() || !port.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(PORT_ERROR.to_owned());
    }
    let port = port.parse::<u16>().map_err(|_| PORT_ERROR.to_owned())?;
    if port == 0 {
        return Err(PORT_ERROR.to_owned());
    }

    let started = Instant::now();
    let result = INFRASTRUCTURE.probe_tcp_port(host.clone(), port).await;
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    match result {
        Ok(()) => Ok(PortTestResult {
            host,
            port,
            reachable: true,
            elapsed_ms,
            error: None,
        }),
        Err(error) => Ok(PortTestResult {
            host,
            port,
            reachable: false,
            elapsed_ms,
            error: Some(error),
        }),
    }
}
