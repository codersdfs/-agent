mod commands;
mod pipeline;

use std::sync::{Arc, Mutex};

pub struct AppState {
    pub pipeline: Arc<tokio::sync::Mutex<pipeline::PipelineState>>,
    pub provider_config: Mutex<providers::ProviderConfig>,
    pub review_config: Mutex<pipeline::ReviewConfig>,
    pub rules_db: Mutex<harness::rules::RulesDatabase>,
    pub detected_language: Mutex<harness::Language>,
    pub db_path: String,
}

impl AppState {
    pub fn new() -> Self {
        let task_id = uuid::Uuid::new_v4().to_string();
        Self {
            pipeline: Arc::new(tokio::sync::Mutex::new(pipeline::PipelineState::new(task_id))),
            provider_config: Mutex::new(providers::ProviderConfig::default()),
            review_config: Mutex::new(pipeline::ReviewConfig::default()),
            rules_db: Mutex::new(harness::rules::RulesDatabase::new()),
            detected_language: Mutex::new(harness::Language::TypeScriptReact),
            db_path: String::new(),
        }
    }
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
            commands::chat::send_message,
            commands::chat::stream_message,
            commands::chat::list_models,
            commands::tools::execute_tool,
            commands::tools::list_tools,
            commands::gate::check_gate,
            commands::gate::get_rules,
            commands::gate::reset_rules,
            commands::gate::set_review_mode,
            commands::tables::query_table,
            commands::memory::memory_store,
            commands::memory::memory_search,
            commands::mcp::mcp_invoke,
            commands::mcp::list_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
