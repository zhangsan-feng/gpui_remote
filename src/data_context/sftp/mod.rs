mod core;
mod query;
mod snapshot;
mod state;

pub(crate) use core::SftpModel;

use super::DataContext;

impl DataContext {
    pub(crate) fn sftp_model(&'static self, workspace_id: String) -> SftpModel {
        SftpModel::new(workspace_id, self)
    }
}
