use crate::{application::ApplicationResult, infrastructure::INFRASTRUCTURE};

use super::{McpApplication, core::normalize_settings, model::McpSettings};

impl McpApplication {
    pub(crate) fn current_settings(&self) -> McpSettings {
        INFRASTRUCTURE.mcp_settings()
    }

    pub(crate) async fn update_settings(
        &self,
        mut settings: McpSettings,
    ) -> ApplicationResult<McpSettings> {
        normalize_settings(&mut settings)?;
        INFRASTRUCTURE.save_mcp_settings(settings).await
    }
}
