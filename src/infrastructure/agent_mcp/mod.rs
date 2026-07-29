mod auth;
mod server;
mod tools;

use crate::application::agent_mcp::AgentMcpClient;

pub(crate) fn start(client: AgentMcpClient) {
    tokio::spawn(async move {
        if let Err(error) = server::run(client).await {
            log::error!("Agent MCP server stopped: {error:#}");
        }
    });
}
