use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use crate::AppState;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatTokenPayload {
    pub message_id: String,
    pub token: String,
    pub done: bool,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    request: SendMessageRequest,
) -> Result<SendMessageResponse, String> {
    log::info!("send_message: agent={}, content={:?}", request.agent_type, request.content.chars().take(50).collect::<String>());

    let config = request.provider.unwrap_or_else(|| {
        let s = state.provider_config.lock().unwrap();
        s.clone()
    });

    let provider = providers::create_provider(&config)?;

    let messages = vec![
        providers::ChatMessage {
            role: "user".into(),
            content: request.content.clone(),
        },
    ];

    let chat_request = providers::ChatRequest {
        messages,
        config,
        stream: false,
    };

    let response = provider.chat(chat_request).await?;

    Ok(SendMessageResponse {
        message_id: uuid::Uuid::new_v4().to_string(),
        content: response.content,
        agent_type: request.agent_type,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamMessageRequest {
    pub content: String,
    pub agent_type: String,
    pub provider: Option<providers::ProviderConfig>,
    pub system_prompt: Option<String>,
}

#[tauri::command]
pub async fn stream_message(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    request: StreamMessageRequest,
) -> Result<String, String> {
    let message_id = uuid::Uuid::new_v4().to_string();
    log::info!("stream_message: id={}, agent={}", message_id, request.agent_type);

    let config = request.provider.unwrap_or_else(|| {
        let s = state.provider_config.lock().unwrap();
        s.clone()
    });

    let provider = providers::create_provider(&config)?;

    let mut messages = vec![];
    if let Some(system) = request.system_prompt {
        messages.push(providers::ChatMessage {
            role: "system".into(),
            content: system,
        });
    }
    messages.push(providers::ChatMessage {
        role: "user".into(),
        content: request.content,
    });

    let chat_request = providers::ChatRequest {
        messages,
        config,
        stream: true,
    };

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let handle = app_handle.clone();
    let mid1 = message_id.clone();
    let mid2 = message_id.clone();

    tokio::spawn(async move {
        if let Err(e) = provider.chat_stream(chat_request, tx).await {
            let _ = handle.emit("chat-error", serde_json::json!({
                "message_id": mid1,
                "error": e,
            }));
        }
    });

    tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let _ = app_handle.emit("chat-token", ChatTokenPayload {
                message_id: mid2.clone(),
                token: chunk.content,
                done: chunk.done,
                model: chunk.model,
            });
            if chunk.done {
                break;
            }
        }
    });

    Ok(message_id)
}

#[tauri::command]
pub async fn list_models(config: providers::ProviderConfig) -> Result<Vec<String>, String> {
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
        _ => Ok(vec!["unknown".into()]),
    }
}
