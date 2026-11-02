use crate::application::{APPLICATION, ApplicationResult};

/// Run an owned application operation on Tokio while GPUI awaits its result.
pub(crate) async fn run_application<T: Send + 'static>(
    operation: impl std::future::Future<Output = ApplicationResult<T>> + Send + 'static,
) -> ApplicationResult<T> {
    tokio::spawn(async move {
        let _accepted = APPLICATION.begin_operation().await?;
        operation.await
    })
    .await
    .map_err(|error| format!("应用层后台任务失败: {error}"))?
}
