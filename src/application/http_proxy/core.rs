use std::net::IpAddr;

use crate::{
    application::ApplicationResult,
    data_context::DATA_CONTEXT,
    domain::http_proxy::{HttpProxySettings, HttpProxyStatus},
};

pub(super) fn normalize_and_validate_settings(
    settings: &mut HttpProxySettings,
) -> ApplicationResult<()> {
    settings.host = settings.host.trim().to_owned();
    let host = settings
        .host
        .parse::<IpAddr>()
        .map_err(|_| "HTTP 代理监听地址必须是 IP 地址".to_owned())?;
    if settings.port == 0 {
        return Err("HTTP 代理监听端口必须在 1-65535 之间".to_owned());
    }
    if settings.username.is_empty() != settings.password.is_empty() {
        return Err("HTTP 代理用户名和密码必须同时填写，或同时留空".to_owned());
    }
    if settings.username.contains(':') {
        return Err("HTTP Basic 用户名不能包含冒号".to_owned());
    }
    if !host.is_loopback() && settings.username.is_empty() {
        return Err("HTTP 代理非回环监听必须配置用户名和密码".to_owned());
    }
    Ok(())
}

pub(super) fn publish_error(error: String) -> String {
    DATA_CONTEXT.set_http_proxy_status(HttpProxyStatus {
        error: Some(error.clone()),
        ..Default::default()
    });
    error
}
