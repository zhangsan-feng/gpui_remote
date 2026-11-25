use std::{convert::Infallible, time::Duration};

use crate::{
    application::ports::SshShell, data_context::DATA_CONTEXT, domain::host_info::HostInfoState,
};

pub(super) async fn monitor_host_info(workspace_id: &str, shell: &dyn SshShell) -> Infallible {
    let mut next_probe = tokio::time::Instant::now();
    let mut selected_before = false;
    loop {
        let selected = DATA_CONTEXT.is_selected_workspace(workspace_id);
        if selected && (!selected_before || next_probe <= tokio::time::Instant::now()) {
            let (info, delay) = match shell.host_info().await {
                Ok(info) => (HostInfoState::Available(info), Duration::from_secs(10)),
                Err(error) => {
                    log::debug!(
                        "SSH host info collection failed: workspace_id={workspace_id}, error={error:#}"
                    );
                    (HostInfoState::Unavailable, Duration::from_secs(30))
                }
            };
            DATA_CONTEXT.publish_host_info(workspace_id, info);
            next_probe = tokio::time::Instant::now() + delay;
        }
        selected_before = selected;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
