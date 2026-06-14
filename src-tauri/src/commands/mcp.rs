use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct McpInvokeRequest {
    pub skill: String,
    pub params: serde_json::Value,
}

#[tauri::command]
pub async fn mcp_invoke(request: McpInvokeRequest) -> Result<serde_json::Value, String> {
    log::info!("mcp_invoke: skill={}", request.skill);
    Ok(serde_json::json!({"status": "not_implemented"}))
}

#[tauri::command]
pub async fn list_skills() -> Result<Vec<String>, String> {
    Ok(vec![])
}
