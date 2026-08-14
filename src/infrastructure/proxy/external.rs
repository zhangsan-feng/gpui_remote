use anyhow::Result;

pub async fn connect(
    target: (&str, u16),
    proxy: Option<&super::ProxySettings>,
) -> Result<Box<dyn super::ProxyStream>> {
    super::core::connect(target, proxy).await
}
