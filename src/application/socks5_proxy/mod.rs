mod core;
mod external;

#[derive(Clone, Copy, Default)]
pub(crate) struct Socks5ProxyApplication;

impl Socks5ProxyApplication {
    pub(crate) fn new() -> Self {
        Self
    }
}
