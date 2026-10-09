use crate::domain::session::SessionProfile;

use super::model::{ProfileSummary, SessionSummary};

pub(crate) fn map_profile_summary(profile: SessionProfile) -> ProfileSummary {
    ProfileSummary {
        id: profile.id,
        title: profile.name,
        host: profile.host,
        protocol: profile.connection_protocol.as_str().to_owned(),
    }
}

pub(crate) fn map_session_summary(
    profile: SessionProfile,
    ssh_tunnel_active: bool,
) -> SessionSummary {
    SessionSummary {
        id: profile.id,
        name: profile.name,
        host: profile.host,
        port: profile.port,
        username: profile.username,
        connection_protocol: profile.connection_protocol,
        ssh_tunnel_configured: profile.ssh_reverse_tunnel.is_some(),
        ssh_tunnel_active,
    }
}
