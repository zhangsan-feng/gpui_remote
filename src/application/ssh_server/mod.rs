mod core;
mod external;

#[derive(Clone, Copy, Default)]
pub(crate) struct SshServerApplication;

impl SshServerApplication {
    pub(crate) fn new() -> Self {
        Self
    }
}
