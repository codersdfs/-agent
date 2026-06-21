use crate::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};

pub struct GoogleProvider {
    #[allow(dead_code)]
    api_key: String,
    #[allow(dead_code)]
    base_url: String,
}

impl GoogleProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".into()),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleProvider {
    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, String> {
        Err("Google provider not yet implemented".into())
    }

    async fn chat_stream(&self, _request: ChatRequest, _tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>) -> Result<(), String> {
        Err("Google streaming not yet implemented".into())
    }
}
