use yal_core::PromptResponse;

pub type RequestId = String;

pub trait Backend: Send + Sync + Clone + 'static {
    fn prompt(
        &self,
        prompt: yal_core::Prompt,
    ) -> impl Future<Output = anyhow::Result<RequestId>> + Send;
    fn prompt_state(
        &self,
        id: RequestId,
    ) -> impl Future<Output = anyhow::Result<PromptResponse>> + Send;
    fn prompt_submission(
        &self,
        id: RequestId,
    ) -> impl Future<Output = anyhow::Result<PromptResponse>> + Send;
    fn prompt_cancel(&self, id: RequestId) -> impl Future<Output = anyhow::Result<()>> + Send;
    fn set_visibility(&self, visible: bool) -> impl Future<Output = anyhow::Result<()>> + Send;
}
