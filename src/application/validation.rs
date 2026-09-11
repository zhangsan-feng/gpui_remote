use crate::{application::ApplicationContext, domain::session::Protocol};

use super::ApplicationResult;

pub(crate) fn selected_sftp_workspace(
    application: &ApplicationContext,
) -> ApplicationResult<String> {
    let workspace_id = application
        .sessions()
        .selected_id()
        .ok_or_else(|| "当前没有选中的 SFTP 会话".to_owned())?;
    validate_session_protocol(application, &workspace_id, Protocol::Sftp)?;
    Ok(workspace_id)
}

pub(crate) fn resolve_terminal_id(
    application: &ApplicationContext,
    workspace_id: Option<String>,
) -> ApplicationResult<String> {
    let workspace_id = workspace_id
        .or_else(|| application.sessions().selected_id())
        .ok_or_else(|| "当前没有选中的终端会话".to_owned())?;
    validate_session_protocol(application, &workspace_id, Protocol::Ssh)?;
    application
        .ssh()
        .snapshot(&workspace_id)
        .map(|_| workspace_id)
}

pub(crate) fn validate_session(
    application: &ApplicationContext,
    workspace_id: &str,
    protocol: Protocol,
    ip: &str,
    title: &str,
) -> ApplicationResult<()> {
    let profile = application
        .sessions()
        .get(workspace_id)
        .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
    if profile.protocol != protocol {
        return Err(format!("会话协议不是 {protocol}: {workspace_id}"));
    }
    if profile.host != ip || profile.name != title {
        return Err(format!(
            "会话信息不匹配: {workspace_id}，请确认 ip 和 title"
        ));
    }
    Ok(())
}

pub(crate) fn validate_session_protocol(
    application: &ApplicationContext,
    workspace_id: &str,
    protocol: Protocol,
) -> ApplicationResult<()> {
    let profile = application
        .sessions()
        .get(workspace_id)
        .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
    if profile.protocol != protocol {
        return Err(format!("会话协议不是 {protocol}: {workspace_id}"));
    }
    Ok(())
}
