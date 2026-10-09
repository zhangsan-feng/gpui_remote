use std::{future::Future, pin::Pin};

use crate::application::mcp::McpSettings;

pub(crate) type McpSettingsFuture =
    Pin<Box<dyn Future<Output = Result<McpSettings, String>> + Send>>;
