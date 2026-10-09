mod core;
mod external;

#[derive(Clone, Copy, Default)]
pub(crate) struct HttpProxyApplication;

impl HttpProxyApplication {
    pub(crate) fn new() -> Self {
        Self
    }
}
