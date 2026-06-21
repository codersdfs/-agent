pub mod commands;
pub mod pipeline;
pub mod tui;

use pipeline::build::BuildSessionEntry;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tauri::Emitter;

// ─── ChatEmitter Trait ────────────────────────────────────────────────────────

/// Abstraction over where chat tokens get written.
/// CLI uses TerminalPrinter (print! to stdout), Tauri uses TauriEmitter (events).
pub trait ChatEmitter: Send + Sync {
    fn emit_token(&self, token: &str) -> Result<(), String>;
    fn emit_done(&self, full: &str) -> Result<(), String>;
    fn emit_error(&self, error: &str) -> Result<(), String>;
}

/// CLI emitter — prints tokens to stdout (existing REPL behaviour).
pub struct TerminalPrinter;
impl ChatEmitter for TerminalPrinter {
    fn emit_token(&self, token: &str) -> Result<(), String> {
        print!("{}", token);
        use std::io::Write;
        std::io::stdout().flush().map_err(|e| e.to_string())
    }
    fn emit_done(&self, _full: &str) -> Result<(), String> {
        println!();
        Ok(())
    }
    fn emit_error(&self, error: &str) -> Result<(), String> {
        eprintln!("{}", error);
        Ok(())
    }
}

/// Tauri emitter — forwards tokens as events to the webview.
pub struct TauriEmitter {
    app_handle: tauri::AppHandle,
}
impl ChatEmitter for TauriEmitter {
    fn emit_token(&self, token: &str) -> Result<(), String> {
        self.app_handle
            .emit("chat-token", token)
            .map_err(|e| e.to_string())
    }
    fn emit_done(&self, full: &str) -> Result<(), String> {
        self.app_handle
            .emit("chat-done", full)
            .map_err(|e| e.to_string())
    }
    fn emit_error(&self, error: &str) -> Result<(), String> {
        self.app_handle
            .emit("chat-error", error)
            .map_err(|e| e.to_string())
    }
}

// ─── Permission Event ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionEvent {
    pub request_id: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub reason: String,
    pub step_id: u32,
    pub step_description: String,
}

// ─── AppState ─────────────────────────────────────────────────────────────────

pub struct AppState {
    pub pipeline: Arc<tokio::sync::Mutex<pipeline::PipelineState>>,
    pub provider_config: Mutex<providers::ProviderConfig>,
    pub review_config: Mutex<pipeline::ReviewConfig>,
    pub rules_db: Mutex<harness::rules::RulesDatabase>,
    pub detected_language: Mutex<harness::Language>,
    pub db_path: String,
    pub build_config: Mutex<pipeline::BuildConfig>,
    pub pending_permissions: Mutex<HashSet<String>>,
    pub permission_results: Mutex<HashMap<String, bool>>,
    pub session_log: Mutex<Vec<BuildSessionEntry>>,
    pub memory_store: Mutex<memory::MemoryStore>,
    /// Broadcast channel for permission requests (Tauri forwards to frontend).
    pub permission_tx: tokio::sync::broadcast::Sender<PermissionEvent>,
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        Self::new_with_provider_config(db_path, providers::ProviderConfig::default())
    }

    pub fn new_with_provider_config(db_path: &str, provider_config: providers::ProviderConfig) -> Self {
        let task_id = uuid::Uuid::new_v4().to_string();
        let memory_store =
            memory::MemoryStore::new(db_path).expect("Failed to initialise memory store");
        let (permission_tx, _) = tokio::sync::broadcast::channel(32);
        Self {
            pipeline: Arc::new(tokio::sync::Mutex::new(pipeline::PipelineState::new(
                task_id,
            ))),
            provider_config: Mutex::new(provider_config),
            review_config: Mutex::new(pipeline::ReviewConfig::default()),
            rules_db: Mutex::new(harness::rules::RulesDatabase::new()),
            detected_language: Mutex::new(harness::Language::TypeScriptReact),
            db_path: db_path.to_string(),
            build_config: Mutex::new(pipeline::BuildConfig::default()),
            pending_permissions: Mutex::new(HashSet::new()),
            permission_results: Mutex::new(HashMap::new()),
            session_log: Mutex::new(vec![]),
            memory_store: Mutex::new(memory_store),
            permission_tx,
        }
    }
}

pub fn default_db_path() -> String {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omega", "omega-agent") {
        let data_dir = proj_dirs.data_dir();
        let path = data_dir.join("memory.db");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        path.to_string_lossy().to_string()
    } else {
        let path = std::path::PathBuf::from(".").join("memory.db");
        path.to_string_lossy().to_string()
    }
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn cmd_send_message(
    state: tauri::State<'_, AppState>,
    request: commands::chat::SendMessageRequest,
) -> Result<commands::chat::SendMessageResponse, String> {
    commands::chat::send_message(&state, request).await
}

#[tauri::command]
async fn cmd_stream_message(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: commands::chat::StreamMessageRequest,
) -> Result<String, String> {
    let emitter = TauriEmitter { app_handle };
    commands::chat::stream_message(&state, request, &emitter).await
}

#[tauri::command]
async fn cmd_list_models(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let config = state.provider_config.lock().unwrap().clone();
    commands::chat::list_models(&config)
}

#[tauri::command]
async fn cmd_execute_tool(
    state: tauri::State<'_, AppState>,
    request: commands::tools::ToolRequest,
) -> Result<commands::tools::ToolResult, String> {
    commands::tools::execute_tool(&state, request).await
}

#[tauri::command]
async fn cmd_list_tools() -> Result<Vec<String>, String> {
    commands::tools::list_tools()
}

#[tauri::command]
async fn cmd_generate_plan(
    state: tauri::State<'_, AppState>,
    task: String,
) -> Result<commands::plan_cmd::PlanGeneratedPayload, String> {
    commands::plan_cmd::generate_plan(&state, task).await
}

#[tauri::command]
async fn cmd_get_plan(
    state: tauri::State<'_, AppState>,
) -> Result<Option<pipeline::plan::StructuredPlan>, String> {
    commands::plan_cmd::get_plan(&state).await
}

#[tauri::command]
async fn cmd_approve_plan(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    commands::plan_cmd::approve_plan(&state).await
}

#[tauri::command]
async fn cmd_get_plan_system_prompt() -> Result<String, String> {
    commands::plan_cmd::get_plan_system_prompt()
}

#[tauri::command]
async fn cmd_execute_build(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BuildSessionEntry>, String> {
    // Forward permission requests to the frontend
    let mut rx = state.permission_tx.subscribe();
    let handle = app_handle.clone();
    tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let _ = handle.emit("permission-request", &event);
        }
    });
    commands::build_cmd::execute_build(&state).await
}

#[tauri::command]
async fn cmd_respond_permission(
    state: tauri::State<'_, AppState>,
    request_id: String,
    approved: bool,
) -> Result<String, String> {
    commands::build_cmd::respond_permission(&state, request_id, approved).await
}

#[tauri::command]
async fn cmd_get_build_session(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BuildSessionEntry>, String> {
    commands::build_cmd::get_build_session(&state).await
}

#[tauri::command]
async fn cmd_get_build_config(
    state: tauri::State<'_, AppState>,
) -> Result<commands::build_cmd::BuildConfigResponse, String> {
    commands::build_cmd::get_build_config(&state).await
}

#[tauri::command]
async fn cmd_set_build_config(
    state: tauri::State<'_, AppState>,
    auto_approve: bool,
) -> Result<String, String> {
    commands::build_cmd::set_build_config(&state, auto_approve).await
}

#[tauri::command]
async fn cmd_run_review(
    state: tauri::State<'_, AppState>,
    request: commands::review_cmd::ReviewRequest,
) -> Result<pipeline::review::CombinedReviewOutput, String> {
    commands::review_cmd::run_review(&state, request).await
}

#[tauri::command]
async fn cmd_get_score_breakdown(
    state: tauri::State<'_, AppState>,
) -> Result<commands::review_cmd::ScoreResponse, String> {
    commands::review_cmd::get_score_breakdown(&state).await
}

#[tauri::command]
async fn cmd_get_promotion_stats(
    state: tauri::State<'_, AppState>,
) -> Result<pipeline::review_score::PromotionStats, String> {
    commands::review_cmd::get_promotion_stats(&state).await
}

#[tauri::command]
async fn cmd_demote_stale_rules(
    state: tauri::State<'_, AppState>,
) -> Result<usize, String> {
    commands::review_cmd::demote_stale_rules(&state).await
}

#[tauri::command]
async fn cmd_reset_retry_count(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    commands::review_cmd::reset_retry_count(&state).await
}

#[tauri::command]
async fn cmd_check_gate(
    state: tauri::State<'_, AppState>,
    request: commands::gate::GateCheckRequest,
) -> Result<commands::tools::GateCheckResult, String> {
    commands::gate::check_gate(&state, request).await
}

#[tauri::command]
async fn cmd_get_rules(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    commands::gate::get_rules(&state).await
}

#[tauri::command]
async fn cmd_reset_rules(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    commands::gate::reset_rules(&state).await
}

#[tauri::command]
async fn cmd_set_review_mode(
    state: tauri::State<'_, AppState>,
    mode: String,
) -> Result<String, String> {
    commands::gate::set_review_mode(&state, mode).await
}

#[tauri::command]
async fn cmd_memory_store(
    state: tauri::State<'_, AppState>,
    request: commands::memory::MemoryStoreRequest,
) -> Result<String, String> {
    commands::memory::memory_store(&state, request).await
}

#[tauri::command]
async fn cmd_memory_search(
    state: tauri::State<'_, AppState>,
    request: commands::memory::MemorySearchRequest,
) -> Result<commands::memory::MemorySearchResponse, String> {
    commands::memory::memory_search(&state, request).await
}

#[tauri::command]
async fn cmd_memory_remember(
    state: tauri::State<'_, AppState>,
    key: String,
    layer: Option<String>,
) -> Result<Option<String>, String> {
    commands::memory::memory_remember(&state, key, layer).await
}

#[tauri::command]
async fn cmd_memory_count(
    state: tauri::State<'_, AppState>,
    layer: Option<String>,
) -> Result<usize, String> {
    commands::memory::memory_count(&state, layer).await
}

#[tauri::command]
async fn cmd_memory_delete(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    commands::memory::memory_delete(&state, id).await
}

#[tauri::command]
async fn cmd_get_provider_config(
    state: tauri::State<'_, AppState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let config = state.provider_config.lock().unwrap();
    let mut map = std::collections::HashMap::new();
    map.insert("provider".into(), config.kind.to_string());
    map.insert("model".into(), config.model.clone());
    map.insert("max_tokens".into(), config.max_tokens.to_string());
    map.insert("temperature".into(), config.temperature.to_string());
    map.insert("base_url".into(), config.base_url.clone().unwrap_or_default());
    map.insert("api_key".into(), if config.api_key.as_deref().unwrap_or("").is_empty() { String::new() } else { "****".into() });
    Ok(map)
}

#[tauri::command]
async fn cmd_list_providers() -> Result<Vec<String>, String> {
    Ok(providers::ProviderKind::all()
        .iter()
        .map(|p| p.to_string())
        .collect())
}

#[tauri::command]
async fn cmd_fetch_models(base_url: String) -> Result<Vec<String>, String> {
    let config = providers::ProviderConfig {
        base_url: Some(base_url),
        ..providers::ProviderConfig::default()
    };
    let models = providers::fetch_models(&config).await?;
    Ok(models.into_iter().map(|m| m.id).collect())
}

#[tauri::command]
async fn cmd_set_provider_config(
    state: tauri::State<'_, AppState>,
    key: String,
    value: String,
) -> Result<String, String> {
    let mut config = state.provider_config.lock().unwrap();
    match key.as_str() {
        "provider" => config.kind = providers::ProviderKind::from_str(&value),
        "model" => config.model = value.clone(),
        "base_url" => config.base_url = Some(value.clone()),
        "api_key" => config.api_key = Some(value.clone()),
        "max_tokens" => config.max_tokens = value.parse().map_err(|_| "max_tokens must be a number".to_string())?,
        "temperature" => config.temperature = value.parse().map_err(|_| "temperature must be a number".to_string())?,
        _ => return Err(format!("Unknown config key: {}. Try: provider, model, base_url, api_key, max_tokens, temperature", key)),
    }
    Ok(format!("{} set to {}", key, value))
}

#[tauri::command]
async fn cmd_memory_clear(
    state: tauri::State<'_, AppState>,
    layer: Option<String>,
) -> Result<usize, String> {
    commands::memory::memory_clear(&state, layer).await
}

// ─── Tauri Entry ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = default_db_path();
    let app_state = AppState::new(&db_path);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            cmd_send_message,
            cmd_stream_message,
            cmd_list_models,
            cmd_list_providers,
            cmd_fetch_models,
            cmd_execute_tool,
            cmd_list_tools,
            cmd_generate_plan,
            cmd_get_plan,
            cmd_approve_plan,
            cmd_get_plan_system_prompt,
            cmd_execute_build,
            cmd_respond_permission,
            cmd_get_build_session,
            cmd_get_build_config,
            cmd_set_build_config,
            cmd_run_review,
            cmd_get_score_breakdown,
            cmd_get_promotion_stats,
            cmd_demote_stale_rules,
            cmd_reset_retry_count,
            cmd_check_gate,
            cmd_get_rules,
            cmd_reset_rules,
            cmd_set_review_mode,
            cmd_get_provider_config,
            cmd_set_provider_config,
            cmd_memory_store,
            cmd_memory_search,
            cmd_memory_remember,
            cmd_memory_count,
            cmd_memory_delete,
            cmd_memory_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
