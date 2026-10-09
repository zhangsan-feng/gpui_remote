use crate::{
    application::{ApplicationResult, ssh::validation::validate_terminal_workspace},
    domain::session::ConnectionProtocol,
    infrastructure::INFRASTRUCTURE,
};

use super::core::SshApplication;

impl SshApplication {
    pub(crate) async fn open_reverse_tunnel(&self, profile_id: String) -> ApplicationResult<()> {
        let profile = self.sessions.find_profile(profile_id).await?;
        if profile.connection_protocol != ConnectionProtocol::SshAndSftp {
            return Err("SSH 隧道仅支持 SSH/SFTP 连接".to_owned());
        }
        let Some(tunnel) = profile.ssh_reverse_tunnel.as_ref() else {
            return Err("请先在连接配置的 SSH 隧道页填写转发参数".to_owned());
        };
        if tunnel.remote_port == 0 {
            return Err("SSH 隧道远端端口必须大于 0".to_owned());
        }
        if tunnel.forward_address.trim().is_empty() {
            return Err("请输入 SSH 隧道转发地址".to_owned());
        }
        if tunnel.forward_port == 0 {
            return Err("SSH 隧道转发端口必须大于 0".to_owned());
        }
        INFRASTRUCTURE
            .open_ssh_reverse_tunnel(&profile)
            .await
            .map_err(|error| format!("开启 SSH 隧道失败: {error:#}"))
    }

    pub(crate) async fn close_reverse_tunnel(&self, profile_id: String) -> ApplicationResult<()> {
        self.sessions.find_profile(profile_id.clone()).await?;
        INFRASTRUCTURE.close_ssh_reverse_tunnel(&profile_id).await;
        Ok(())
    }

    pub(crate) async fn send_text(
        &self,
        workspace_id: String,
        text: String,
    ) -> ApplicationResult<()> {
        validate_terminal_workspace(&self.sessions, &workspace_id)?;
        self.send_input(&workspace_id, text.into_bytes()).await
    }

    pub(crate) async fn send_key_checked(
        &self,
        workspace_id: String,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> ApplicationResult<()> {
        validate_terminal_workspace(&self.sessions, &workspace_id)?;
        self.send_key(&workspace_id, &key, control, alt, shift)
            .await
    }
}
