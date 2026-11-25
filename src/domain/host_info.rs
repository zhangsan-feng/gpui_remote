#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum HostInfoState {
    #[default]
    Pending,
    Available(RemoteHostInfo),
    Unavailable,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RemoteHostInfo {
    pub(crate) system: Option<String>,
    pub(crate) cpu: Option<String>,
    pub(crate) logical_cpus: Option<u32>,
    pub(crate) memory: Option<HostMemory>,
    pub(crate) load: Option<[f32; 3]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HostMemory {
    pub(crate) total_kib: u64,
    pub(crate) available_kib: u64,
}
