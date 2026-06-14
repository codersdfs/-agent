use serde::{Deserialize, Serialize};
use harness::GateResult;

#[derive(Debug, Serialize, Deserialize)]
pub struct GateCheckRequest {
    pub content: String,
    pub context: String,
}

#[tauri::command]
pub async fn check_gate(request: GateCheckRequest) -> Result<GateResult, String> {
    log::info!("check_gate: content_len={}", request.content.len());
    // TODO: run full gate check
    Ok(GateResult::pass())
}

#[tauri::command]
pub async fn get_rules() -> Result<Vec<String>, String> {
    Ok(vec!["structural".into(), "taste".into(), "golden".into()])
}
