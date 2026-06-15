use crate::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};

pub struct MistralProvider;

impl MistralProvider {
    pub fn new(_api_key: String, _base_url: Option<String>) -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LlmProvider for MistralProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err("Mistral provider not yet implemented".into())
    }

    async fn chat_stream(&self, _request: ChatRequest, _tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>) -> Result<(), String> {
        Err("Mistral streaming not yet implemented".into())
    }
}
