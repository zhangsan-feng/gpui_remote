use std::sync::Arc;

use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::TerminalStatus,
};

use super::{DatabaseApplication, DatabaseApplicationInner};
use crate::{data_context::DATA_CONTEXT, infrastructure::INFRASTRUCTURE};

impl DatabaseApplication {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(DatabaseApplicationInner {
                connections: std::sync::RwLock::new(std::collections::HashMap::new()),
            }),
        }
    }

    pub(crate) async fn open(
        &self,
        workspace_id: String,
        profile: SessionProfile,
    ) -> Result<(), String> {
        if !matches!(
            profile.protocol,
            Protocol::Mysql | Protocol::Pgsql | Protocol::Redis
        ) {
            return Err(format!("数据库模块不支持 {} 协议", profile.protocol));
        }
        if !profile.connection_protocol.supports(profile.protocol) {
            return Err(format!(
                "连接类型 {} 与工作区协议 {} 不匹配",
                profile.connection_protocol, profile.protocol
            ));
        }
        {
            let connections = self
                .inner
                .connections
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if connections.contains_key(&workspace_id) {
                return Err(format!("数据库工作区已打开: {workspace_id}"));
            }
        }
        let connection = INFRASTRUCTURE
            .connect_database(&profile)
            .await
            .map_err(|error| format!("建立数据库连接失败: {error:#}"))?;
        let connection: Arc<dyn crate::application::ports::DatabaseConnection> =
            Arc::from(connection);
        let mut connection = Some(connection);
        let duplicate = {
            let mut connections = self
                .inner
                .connections
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if connections.contains_key(&workspace_id) {
                true
            } else {
                connections.insert(
                    workspace_id.clone(),
                    connection.take().expect("数据库连接应在插入前仍然存在"),
                );
                false
            }
        };
        if duplicate {
            let connection = connection.take().expect("重复数据库连接应在关闭前仍然存在");
            if let Err(error) = connection.close().await {
                log::warn!("关闭重复数据库连接失败: workspace_id={workspace_id}, error={error:#}");
            }
            return Err(format!("数据库工作区已打开: {workspace_id}"));
        }
        DATA_CONTEXT.update_database_workspace(&workspace_id, TerminalStatus::Connected);
        Ok(())
    }

    pub(crate) async fn close(&self, workspace_id: &str) -> Result<(), String> {
        let connection = self
            .inner
            .connections
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(workspace_id)
            .ok_or_else(|| format!("数据库会话不存在: {workspace_id}"))?;
        let close_result = connection
            .close()
            .await
            .map_err(|error| format!("关闭数据库连接失败: {error:#}"));
        DATA_CONTEXT.update_database_workspace(workspace_id, TerminalStatus::Disconnected);
        close_result
    }
}
