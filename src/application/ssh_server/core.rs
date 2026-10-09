use crate::application::ApplicationResult;

pub(super) fn normalize_and_validate_ssh_server_settings(
    settings: &mut crate::domain::ssh_server::SshServerSettings,
) -> ApplicationResult<()> {
    settings.host = "0.0.0.0".to_owned();
    settings.username = settings.username.trim().to_owned();
    settings.password = settings.password.trim().to_owned();
    if settings.port == 0 {
        return Err("SSH 监听端口必须在 1-65535 之间".to_owned());
    }
    if settings.username.is_empty() {
        return Err("SSH 登录用户名不能为空".to_owned());
    }
    if settings.enabled && settings.password.is_empty() {
        return Err("启用 SSH 服务前需设置登录密码".to_owned());
    }
    Ok(())
}
