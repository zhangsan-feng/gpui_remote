#[derive(Clone, Debug)]
pub(crate) struct PortForwardRuleDraft {
    pub(crate) listen_host: String,
    pub(crate) listen_port: String,
    pub(crate) target_host: String,
    pub(crate) target_port: String,
}

impl Default for PortForwardRuleDraft {
    fn default() -> Self {
        Self {
            listen_host: "0.0.0.0".to_owned(),
            listen_port: "8080".to_owned(),
            target_host: String::new(),
            target_port: String::new(),
        }
    }
}
