mod core;
mod external;
mod model;

pub(crate) use model::PortTestResult;

#[derive(Clone, Copy, Default)]
pub(crate) struct PortTestApplication;

impl PortTestApplication {
    pub(crate) fn new() -> Self {
        Self
    }
}
