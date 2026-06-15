use crate::{ChatRequest, ChatResponse, LlmProvider, StreamChunk};
use serde::Serialize;

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: f32,
    stream: bool,
}

#[derive(serde::Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(serde::Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    model: String,
}

pub struct AnthropicProvider {
    api_key: String,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com/v1".into()),
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let client = reqwest::Client::new();
        let body = AnthropicRequest {
            model: request.config.model,
            messages: request.messages.iter().map(|m| AnthropicMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            max_tokens: request.config.max_tokens,
            temperature: request.config.temperature,
            stream: false,
        };

        let resp = client.post(format!("{}/messages", self.base_url.trim_end_matches('/')))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic error: {}", text));
        }

        let data: AnthropicResponse = resp.json().await.map_err(|e| format!("parse failed: {}", e))?;
        let text: String = data.content.into_iter().map(|c| c.text).collect();

        Ok(ChatResponse {
            content: text,
            model: data.model,
            usage: None,
        })
    }

    async fn chat_stream(&self, _request: ChatRequest, _tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>) -> Result<(), String> {
        Err("streaming not yet implemented for Anthropic".into())
    }
}
