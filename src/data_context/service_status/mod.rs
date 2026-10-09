use std::collections::HashMap;

use crate::domain::{
    http_proxy::HttpProxyStatus, port_forward::PortForwardStatus, socks5_proxy::Socks5ProxyStatus,
    ssh_server::SshServerStatus,
};

use super::{DataContext, DataSnapshot};

impl DataContext {
    pub(crate) fn http_proxy(&self) -> HttpProxyStatus {
        self.read_snapshot(|snapshot| snapshot.http_proxy.clone())
    }

    pub(crate) fn port_forward_statuses(&self) -> HashMap<String, PortForwardStatus> {
        self.read_snapshot(|snapshot| snapshot.port_forward_statuses.clone())
    }

    pub(crate) fn socks5_proxy(&self) -> Socks5ProxyStatus {
        self.read_snapshot(|snapshot| snapshot.socks5_proxy.clone())
    }

    pub(crate) fn ssh_server(&self) -> SshServerStatus {
        self.read_snapshot(|snapshot| snapshot.ssh_server.clone())
    }

    pub(crate) fn set_http_proxy_status(&self, status: HttpProxyStatus) {
        self.commit_http_proxy_status(status);
    }

    fn commit_http_proxy_status(&self, status: HttpProxyStatus) {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.http_proxy == status {
                return;
            }
            snapshot.http_proxy = status;
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
    }

    pub(crate) fn set_port_forward_statuses(&self, statuses: HashMap<String, PortForwardStatus>) {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.port_forward_statuses == statuses {
                return;
            }
            snapshot.port_forward_statuses = statuses;
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
    }

    pub(crate) fn set_socks5_proxy_status(&self, status: Socks5ProxyStatus) {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.socks5_proxy == status {
                return;
            }
            snapshot.socks5_proxy = status;
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
    }

    pub(crate) fn set_ssh_server_status(&self, status: SshServerStatus) {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        {
            let mut snapshot = self
                .snapshot
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if snapshot.ssh_server == status {
                return;
            }
            snapshot.ssh_server = status;
            snapshot.revision = snapshot.revision.wrapping_add(1);
        }
        drop(_commit);
        self.notice.notify_gui_refresh();
    }

    fn read_snapshot<R>(&self, read: impl FnOnce(&DataSnapshot) -> R) -> R {
        let snapshot = self
            .snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        read(&snapshot)
    }
}
