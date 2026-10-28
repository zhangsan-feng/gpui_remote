use std::time::Duration;

impl super::super::InfrastructureContext {
    pub(crate) async fn write_clipboard_text(&self, text: String) -> Result<(), String> {
        let task = tokio::task::spawn_blocking(move || super::core::write_text(text));
        // Win32 calls cannot be cancelled. A slow owner must not hold up the GUI.
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .map_err(|_| "系统剪贴板写入超时，后台操作仍在等待系统返回".to_owned())?
            .map_err(|error| format!("剪贴板写入任务失败: {error}"))?
    }
}
