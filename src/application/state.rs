use gpui_kit::{App, AppContext, Entity};

use super::{SessionApplication, SftpApplication, SshApplication};

pub(crate) struct SessionStore {
    pub(crate) application: SessionApplication,
}

pub(crate) struct SshStore {
    pub(crate) application: SshApplication,
}

pub(crate) struct SftpStore {
    pub(crate) application: SftpApplication,
}

pub(crate) struct ApplicationStoreGraph {
    pub(crate) session: Entity<SessionStore>,
    pub(crate) ssh: Entity<SshStore>,
    pub(crate) sftp: Entity<SftpStore>,
}

impl ApplicationStoreGraph {
    pub(crate) fn new(
        cx: &mut App,
        sessions: SessionApplication,
        ssh: SshApplication,
        sftp: SftpApplication,
    ) -> Self {
        Self {
            session: cx.new(|_| SessionStore {
                application: sessions,
            }),
            ssh: cx.new(|_| SshStore { application: ssh }),
            sftp: cx.new(|_| SftpStore { application: sftp }),
        }
    }
}
