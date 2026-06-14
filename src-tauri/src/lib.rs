mod commands;
mod pipeline;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub pipeline: pipeline::SharedPipelineState,
    pub db_path: String,
}

impl AppState {
    pub fn new() -> Self {
        let task_id = uuid::Uuid::new_v4().to_string();
        Self {
            pipeline: Arc::new(Mutex::new(pipeline::PipelineState::new(task_id))),
            db_path: String::new(),
        }
    }
}

#[tauri::command]
pub fn get_app_info(state: tauri::State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": "Omega Agent",
        "version": "0.1.0",
        "pipeline_status": "idle",
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            commands::chat::send_message,
            commands::chat::stream_message,
            commands::tools::execute_tool,
            commands::tools::list_tools,
            commands::gate::check_gate,
            commands::gate::get_rules,
            commands::tables::query_table,
            commands::memory::memory_store,
            commands::memory::memory_search,
            commands::mcp::mcp_invoke,
            commands::mcp::list_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
