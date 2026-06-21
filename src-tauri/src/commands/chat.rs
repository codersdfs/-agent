use serde::{Deserialize, Serialize};
use crate::AppState;
use crate::ChatEmitter;

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub agent_type: String,
    pub provider: Option<providers::ProviderConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub content: String,
    pub agent_type: String,
}

pub async fn send_message(
    state: &AppState,
    request: SendMessageRequest,
) -> Result<SendMessageResponse, String> {
    log::info!("send_message: agent={}, content={:?}", request.agent_type, request.content.chars().take(50).collect::<String>());

    let config = request.provider.unwrap_or_else(|| {
        let s = state.provider_config.lock().unwrap();
        s.clone()
    });

    let provider = providers::create_provider(&config)?;
    let tools = crate::commands::tools::tool_definitions();

    let mut messages = vec![
        providers::ChatMessage {
            role: "user".into(),
            content: request.content.clone(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        },
    ];

    let mut max_loops = 10;

    loop {
        if max_loops == 0 {
            return Err("Tool call loop exceeded max iterations".into());
        }
        max_loops -= 1;

        let chat_request = providers::ChatRequest {
            messages: messages.clone(),
            config: config.clone(),
            stream: false,
            tools: Some(tools.clone()),
        };

        let response = provider.chat(chat_request).await?;

        if let Some(tool_calls) = response.tool_calls {
            messages.push(providers::ChatMessage {
                role: "assistant".into(),
                content: String::new(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                name: None,
            });

            for tc in &tool_calls {
                let tool_request = crate::commands::tools::ToolRequest {
                    tool: tc.function.name.clone(),
                    args: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null),
                };
                let result = crate::commands::tools::execute_tool_inner(state, tool_request).await?;
                messages.push(providers::ChatMessage {
                    role: "tool".into(),
                    content: result.output,
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    name: Some(tc.function.name.clone()),
                });
            }
        } else {
            return Ok(SendMessageResponse {
                message_id: uuid::Uuid::new_v4().to_string(),
                content: response.content,
                agent_type: request.agent_type,
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessageRequest {
    pub content: String,
    pub agent_type: String,
    pub provider: Option<providers::ProviderConfig>,
    pub system_prompt: Option<String>,
}

/// Stream a message to the LLM, forwarding tokens to the given emitter.
/// Automatically handles tool calling loops — executes tools silently
/// and streams only the final text response from each round.
pub async fn stream_message<E: ChatEmitter>(
    state: &AppState,
    request: StreamMessageRequest,
    emitter: &E,
) -> Result<String, String> {
    log::info!("stream_message: agent={}", request.agent_type);

    let config = request.provider.unwrap_or_else(|| {
        let s = state.provider_config.lock().unwrap();
        s.clone()
    });

    let provider = providers::create_provider(&config)?;
    let tools = crate::commands::tools::tool_definitions();

    let mut messages = vec![];
    if let Some(system) = request.system_prompt {
        messages.push(providers::ChatMessage {
            role: "system".into(),
            content: system,
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });
    }
    messages.push(providers::ChatMessage {
        role: "user".into(),
        content: request.content,
        tool_calls: None,
        tool_call_id: None,
        name: None,
    });

    let mut full_response = String::new();
    let mut max_loops: u32 = 10;

    loop {
        if max_loops == 0 {
            return Err("Tool call loop exceeded max iterations".into());
        }
        max_loops -= 1;

        let chat_request = providers::ChatRequest {
            messages,
            config: config.clone(),
            stream: false,
            tools: Some(tools.clone()),
        };

        let response = provider.chat(chat_request).await?;

        if let Some(tool_calls) = response.tool_calls {
            messages = vec![
                providers::ChatMessage {
                    role: "assistant".into(),
                    content: String::new(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                },
            ];

            for tc in &tool_calls {
                let tool_request = crate::commands::tools::ToolRequest {
                    tool: tc.function.name.clone(),
                    args: serde_json::from_str(&tc.function.arguments)
                        .unwrap_or(serde_json::Value::Null),
                };
                let result = crate::commands::tools::execute_tool_inner(state, tool_request).await?;
                messages.push(providers::ChatMessage {
                    role: "tool".into(),
                    content: result.output,
                    tool_calls: None,
                    tool_call_id: Some(tc.id.clone()),
                    name: Some(tc.function.name.clone()),
                });
            }
        } else {
            if !response.content.is_empty() {
                emitter.emit_token(&response.content)?;
                full_response.push_str(&response.content);
            }
            emitter.emit_done(&full_response)?;
            return Ok(full_response);
        }
    }
}

pub fn list_models(config: &providers::ProviderConfig) -> Result<Vec<String>, String> {
    log::info!("list_models for provider={:?}", config.kind);
    match config.kind {
        providers::ProviderKind::OpenAI => Ok(vec![
            "gpt-4o".into(), "gpt-4o-mini".into(), "gpt-4-turbo".into(), "gpt-3.5-turbo".into(),
        ]),
        providers::ProviderKind::Anthropic => Ok(vec![
            "claude-3-5-sonnet-20241022".into(), "claude-3-5-haiku-20241022".into(),
            "claude-opus-4-20250514".into(),
        ]),
        providers::ProviderKind::Groq => Ok(vec![
            "llama-3.3-70b-versatile".into(), "mixtral-8x7b-32768".into(),
        ]),
        providers::ProviderKind::XAI => Ok(vec![
            "grok-3".into(), "grok-3-mini".into(),
        ]),
        providers::ProviderKind::Local => Ok(vec!["ollama".into()]),
        _ => Ok(vec!["unknown".into()]),
    }
}
