#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PortTestResult {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) reachable: bool,
    pub(crate) elapsed_ms: u64,
    pub(crate) error: Option<String>,
}
