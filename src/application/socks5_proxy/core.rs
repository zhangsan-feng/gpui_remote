use crate::application::ApplicationResult;

pub(super) fn normalize_and_validate_socks5_settings(
    settings: &mut crate::domain::socks5_proxy::Socks5ProxySettings,
) -> ApplicationResult<()> {
    settings.host = settings.host.trim().to_owned();
    settings.username = settings.username.trim().to_owned();
    settings.password = settings.password.trim().to_owned();
    if settings.host.parse::<std::net::IpAddr>().is_err() {
        return Err("SOCKS5 监听地址必须是 IP 地址".to_owned());
    }
    if settings.port == 0 {
        return Err("SOCKS5 监听端口必须在 1-65535 之间".to_owned());
    }
    if settings.username.is_empty() != settings.password.is_empty() {
        return Err("SOCKS5 用户名和密码必须同时填写，或同时留空".to_owned());
    }
    Ok(())
}
