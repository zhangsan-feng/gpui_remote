pub(crate) mod agent_mcp;
mod context;
pub(crate) mod profile_query;
pub mod proxy;
pub mod storage;

pub(crate) use context::InfrastructureContext;

pub(crate) fn new() -> InfrastructureContext {
    InfrastructureContext::new()
}
