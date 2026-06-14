use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub agent_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub content: String,
    pub agent_type: String,
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    request: SendMessageRequest,
) -> Result<SendMessageResponse, String> {
    log::info!("send_message: agent={}, content={:?}", request.agent_type, request.content.chars().take(50).collect::<String>());
    // TODO: route to appropriate agent pipeline
    Ok(SendMessageResponse {
        message_id: uuid::Uuid::new_v4().to_string(),
        content: format!("Echo: {}", request.content),
        agent_type: request.agent_type,
    })
}

#[tauri::command]
pub async fn stream_message(
    app_handle: tauri::AppHandle,
    request: SendMessageRequest,
) -> Result<(), String> {
    // TODO: implement streaming via events
    log::info!("stream_message requested for agent={}", request.agent_type);
    Ok(())
}
