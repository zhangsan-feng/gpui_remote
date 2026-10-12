mod core;
pub(super) mod external;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PortTestProbeResult {
    pub(crate) reachable: bool,
    pub(crate) host_reachable: Option<bool>,
    pub(crate) error: Option<String>,
}
