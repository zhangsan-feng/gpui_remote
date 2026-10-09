use crate::{application::ApplicationResult, domain::session::Protocol};

use super::SessionApplication;

pub(crate) fn validate_session(
    sessions: &SessionApplication,
    workspace_id: &str,
    protocol: Protocol,
    ip: &str,
    title: &str,
) -> ApplicationResult<()> {
    let profile = sessions
        .profile_for_workspace(workspace_id)
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
    sessions: &SessionApplication,
    workspace_id: &str,
    protocol: Protocol,
) -> ApplicationResult<()> {
    let profile = sessions
        .profile_for_workspace(workspace_id)
        .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
    if profile.protocol != protocol {
        return Err(format!("会话协议不是 {protocol}: {workspace_id}"));
    }
    Ok(())
}
