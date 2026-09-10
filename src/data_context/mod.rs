mod bus;
mod command;
mod core;
mod event;
mod gui;
mod infrastructure;
mod mcp;
mod model;
mod query;

pub use command::{DataContextCommand, DataContextResult, SftpCommand, SshCommand};
pub use core::DataContext;
#[allow(unused_imports)]
pub use event::DataContextEvent;
pub use gui::{GuiContext, GuiContextReceiver, gui_context_channel};
pub use infrastructure::InfrastructureContext;
pub use mcp::McpContext;
pub use model::{
    ProfileSummary, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
    SftpWatchSummary, TerminalReadPage, TerminalSummary,
};
pub use query::{ProfileQuery, QueryService};

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::{DataContext, InfrastructureContext, ProfileQuery, ProfileSummary};

    struct StaticProfileQuery;

    impl ProfileQuery for StaticProfileQuery {
        fn list_profiles(&self) -> Result<Vec<ProfileSummary>, String> {
            Ok(vec![ProfileSummary {
                id: "profile-1".to_owned(),
                title: "Local".to_owned(),
                host: "127.0.0.1".to_owned(),
                protocol: "ssh".to_owned(),
            }])
        }
    }

    #[tokio::test]
    async fn mcp_context_queries_without_entering_the_gui_bus() {
        let infrastructure = InfrastructureContext::new(Arc::new(StaticProfileQuery));
        let (data_context, mut gui_receiver) = DataContext::new(infrastructure);

        let profiles = data_context
            .mcp()
            .list_profiles()
            .await
            .expect("profile query should succeed");

        assert_eq!(profiles[0].id, "profile-1");
        assert!(
            tokio::time::timeout(Duration::from_millis(20), gui_receiver.recv())
                .await
                .is_err()
        );
    }
}
