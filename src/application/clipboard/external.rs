impl super::super::Application {
    pub(crate) async fn write_clipboard_text(
        &self,
        text: String,
    ) -> super::super::ApplicationResult<()> {
        crate::infrastructure::INFRASTRUCTURE
            .write_clipboard_text(text)
            .await
    }
}
