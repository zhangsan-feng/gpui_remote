use crate::{application::ApplicationContext, domain::session::Protocol};

use super::ApplicationResult;

pub(crate) fn validate_terminal_workspace(
    application: &ApplicationContext,
    workspace_id: &str,
) -> ApplicationResult<()> {
    validate_session_protocol(application, workspace_id, Protocol::Ssh)?;
    application.ssh().snapshot(workspace_id).map(|_| ())
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
