use std::time::Instant;

use gpui_kit::Context;

use crate::data_context::DataSnapshot;

use super::Workspace;

impl Workspace {
    pub(super) fn sync_from_context(&mut self, cx: &mut Context<Self>) {
        let snapshot = crate::data_context::DATA_CONTEXT.workspace_snapshot();
        self.apply_workspace_snapshot(&snapshot, cx);
    }

    fn apply_workspace_snapshot(&mut self, snapshot: &DataSnapshot, cx: &mut Context<Self>) {
        let apply_started = Instant::now();
        log::debug!(
            "Workspace snapshot apply started: revision={}, workspaces={}, selected_workspace_id={}",
            snapshot.revision,
            snapshot.workspaces.len(),
            snapshot.selected_workspace_id.as_deref().unwrap_or("none")
        );

        let phase_started = Instant::now();
        self.workspace
            .update(cx, |workspace, cx| workspace.apply_snapshot(snapshot, cx));
        log::debug!(
            "Workspace snapshot phase completed: revision={}, phase=workspace_sessions, elapsed_us={}",
            snapshot.revision,
            phase_started.elapsed().as_micros()
        );

        let phase_started = Instant::now();
        self.terminal
            .update(cx, |terminal, cx| terminal.apply_snapshot(snapshot, cx));
        log::debug!(
            "Workspace snapshot phase completed: revision={}, phase=ssh_terminal_projection, elapsed_us={}",
            snapshot.revision,
            phase_started.elapsed().as_micros()
        );

        let phase_started = Instant::now();
        self.sftp
            .update(cx, |sftp, cx| sftp.apply_snapshot(snapshot, cx));
        log::debug!(
            "Workspace snapshot phase completed: revision={}, phase=sftp_projection, elapsed_us={}",
            snapshot.revision,
            phase_started.elapsed().as_micros()
        );

        let phase_started = Instant::now();
        self.select_workspace(snapshot.selected_workspace_id.as_deref(), cx);
        log::debug!(
            "Workspace snapshot phase completed: revision={}, phase=select_workspace, elapsed_us={}",
            snapshot.revision,
            phase_started.elapsed().as_micros()
        );
        log::debug!(
            "Workspace snapshot apply completed: revision={}, elapsed_us={}",
            snapshot.revision,
            apply_started.elapsed().as_micros()
        );
    }
}
