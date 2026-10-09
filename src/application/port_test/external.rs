use crate::{application::ApplicationResult, application::port_test::PortTestResult};

use super::core;

impl super::PortTestApplication {
    pub(crate) async fn test_tcp_port(
        &self,
        host: String,
        port: String,
    ) -> ApplicationResult<PortTestResult> {
        core::test_tcp_port(host, port).await
    }
}
