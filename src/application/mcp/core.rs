use crate::application::ApplicationResult;

use super::model::McpSettings;

pub(super) fn normalize_settings(settings: &mut McpSettings) -> ApplicationResult<()> {
    settings.host = "0.0.0.0".to_owned();
    if settings.port == 0 {
        return Err("MCP Port 必须在 1-65535 之间".to_owned());
    }
    Ok(())
}
