use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreRequest {
    pub key: String,
    pub value: String,
    pub layer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub layer: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResponse {
    pub entries: Vec<memory::MemoryEntry>,
    pub relevance: Vec<f64>,
}

#[tauri::command]
pub async fn memory_store(
    state: State<'_, AppState>,
    request: MemoryStoreRequest,
) -> Result<String, String> {
    log::info!("memory_store: key={}, layer={}", request.key, request.layer);

    let layer = memory::MemoryLayer::from_str(&request.layer);
    let store = state.memory_store.lock().unwrap();
    store.store(layer, &request.key, &request.value)
}

#[tauri::command]
pub async fn memory_search(
    state: State<'_, AppState>,
    request: MemorySearchRequest,
) -> Result<MemorySearchResponse, String> {
    log::info!("memory_search: query={}", request.query);

    let store = state.memory_store.lock().unwrap();
    let result = store.search(&request.query, request.layer.as_deref(), request.limit.unwrap_or(10))?;

    Ok(MemorySearchResponse {
        entries: result.entries,
        relevance: result.relevance,
    })
}

#[tauri::command]
pub async fn memory_remember(
    state: State<'_, AppState>,
    key: String,
    layer: Option<String>,
) -> Result<Option<String>, String> {
    log::info!("memory_remember: key={}", key);

    let store = state.memory_store.lock().unwrap();
    store.remember(&key, layer.as_deref())
}

#[tauri::command]
pub async fn memory_count(
    state: State<'_, AppState>,
    layer: Option<String>,
) -> Result<usize, String> {
    let store = state.memory_store.lock().unwrap();
    store.count(layer.as_deref())
}

#[tauri::command]
pub async fn memory_delete(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let store = state.memory_store.lock().unwrap();
    store.delete(&id)
}

#[tauri::command]
pub async fn memory_clear(
    state: State<'_, AppState>,
    layer: Option<String>,
) -> Result<usize, String> {
    let store = state.memory_store.lock().unwrap();
    store.clear(layer.as_deref())
}
