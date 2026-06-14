use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub value: String,
    pub layer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub layer: Option<String>,
    pub limit: Option<usize>,
}

#[tauri::command]
pub async fn memory_store(request: MemoryStoreRequest) -> Result<String, String> {
    log::info!("memory_store: key={}, layer={}", request.key, request.layer);
    Ok(uuid::Uuid::new_v4().to_string())
}

#[tauri::command]
pub async fn memory_search(request: MemorySearchRequest) -> Result<Vec<serde_json::Value>, String> {
    log::info!("memory_search: query={}", request.query);
    Ok(vec![])
}
