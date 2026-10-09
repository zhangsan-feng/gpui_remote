use crate::application::ApplicationResult;

use super::model::McpSettings;

pub(super) fn normalize_settings(settings: &mut McpSettings) -> ApplicationResult<()> {
    settings.host = settings.host.trim().to_owned();
    if settings.host.is_empty() {
        return Err("MCP Host 不能为空".to_owned());
    }
    if settings.port == 0 {
        return Err("MCP Port 必须在 1-65535 之间".to_owned());
    }
    Ok(())
}
