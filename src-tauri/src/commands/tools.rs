use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn execute_tool(request: ToolRequest) -> Result<ToolResult, String> {
    log::info!("execute_tool: tool={}", request.tool);
    // TODO: route to tool executor
    Ok(ToolResult {
        success: true,
        output: format!("Tool {} executed", request.tool),
        error: None,
    })
}

#[tauri::command]
pub async fn list_tools() -> Result<Vec<String>, String> {
    Ok(vec![
        "read".into(),
        "write".into(),
        "edit".into(),
        "bash".into(),
        "grep".into(),
        "glob".into(),
    ])
}
