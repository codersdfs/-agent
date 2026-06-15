use crate::{ChatRequest, ChatResponse, LlmProvider, StreamChunk, Usage};
use serde::Serialize;

#[derive(Serialize, serde::Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    stream: bool,
    max_tokens: u32,
    temperature: f32,
}

#[derive(serde::Deserialize)]
struct OpenAIResponseChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIResponseChoice>,
    model: String,
    usage: Option<OpenAIUsage>,
}

#[derive(serde::Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(serde::Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(serde::Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct StreamEvent {
    choices: Vec<StreamChoice>,
    model: Option<String>,
}

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }

    fn build_request(&self, request: &ChatRequest) -> OpenAIRequest {
        OpenAIRequest {
            model: request.config.model.clone(),
            messages: request.messages.iter().map(|m| OpenAIMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            }).collect(),
            stream: request.stream,
            max_tokens: request.config.max_tokens,
            temperature: request.config.temperature,
        }
    }

    fn url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let client = reqwest::Client::new();
        let body = self.build_request(&request);

        let resp = client.post(self.url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("API error {}: {}", status, text));
        }

        let data: OpenAIResponse = resp.json().await
            .map_err(|e| format!("parse failed: {}", e))?;

        let choice = data.choices.into_iter().next()
            .ok_or_else(|| "no choices returned".to_string())?;

        Ok(ChatResponse {
            content: choice.message.content,
            model: data.model,
            usage: data.usage.map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
            }),
        })
    }

    async fn chat_stream(&self, request: ChatRequest, tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>) -> Result<(), String> {
        let client = reqwest::Client::new();
        let body = self.build_request(&request);

        let resp = client.post(self.url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("stream request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("stream API error {}: {}", status, text));
        }

        let stream = resp.bytes_stream();
        use futures_util::StreamExt;
        let mut buf = String::new();

        tokio::pin!(stream);
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("stream read error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            buf.push_str(&text);

            while let Some(line_end) = buf.find('\n') {
                let line = buf[..line_end].trim().to_string();
                buf.drain(..=line_end);

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if line == "data: [DONE]" {
                    let _ = tx.send(StreamChunk {
                        content: String::new(),
                        done: true,
                        model: None,
                        usage: None,
                    });
                    return Ok(());
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(event) = serde_json::from_str::<StreamEvent>(data) {
                        if let Some(choice) = event.choices.into_iter().next() {
                            let content = choice.delta.content.unwrap_or_default();
                            let is_done = choice.finish_reason.is_some();
                            let _ = tx.send(StreamChunk {
                                content,
                                done: is_done,
                                model: event.model.clone(),
                                usage: None,
                            });
                            if is_done {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
