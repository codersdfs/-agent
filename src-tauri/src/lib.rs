mod commands;
mod pipeline;

use pipeline::build::BuildSessionEntry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

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
}

impl AppState {
    pub fn new(db_path: &str) -> Self {
        let task_id = uuid::Uuid::new_v4().to_string();
        let memory_store = memory::MemoryStore::new(db_path)
            .expect("Failed to initialize memory store");

        Self {
            pipeline: Arc::new(tokio::sync::Mutex::new(pipeline::PipelineState::new(task_id))),
            provider_config: Mutex::new(providers::ProviderConfig::default()),
            review_config: Mutex::new(pipeline::ReviewConfig::default()),
            rules_db: Mutex::new(harness::rules::RulesDatabase::new()),
            detected_language: Mutex::new(harness::Language::TypeScriptReact),
            db_path: db_path.to_string(),
            build_config: Mutex::new(pipeline::BuildConfig::default()),
            pending_permissions: Mutex::new(HashSet::new()),
            permission_results: Mutex::new(HashMap::new()),
            session_log: Mutex::new(vec![]),
            memory_store: Mutex::new(memory_store),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omega", "omega-agent") {
        let data_dir = proj_dirs.data_dir();
        let path = data_dir.join("memory.db");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        path.to_string_lossy().to_string()
    } else {
        let path = std::path::PathBuf::from(".").join("memory.db");
        path.to_string_lossy().to_string()
    };

    tauri::Builder::default()
        .manage(AppState::new(&db_path))
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
            commands::memory::memory_remember,
            commands::memory::memory_count,
            commands::memory::memory_delete,
            commands::memory::memory_clear,
            commands::mcp::mcp_invoke,
            commands::mcp::list_skills,
            commands::plan_cmd::generate_plan,
            commands::plan_cmd::get_plan,
            commands::plan_cmd::approve_plan,
            commands::plan_cmd::get_plan_system_prompt,
            commands::build_cmd::execute_build,
            commands::build_cmd::respond_permission,
            commands::build_cmd::get_build_session,
            commands::build_cmd::get_build_config,
            commands::build_cmd::set_build_config,
            commands::review_cmd::run_review,
            commands::review_cmd::get_score_breakdown,
            commands::review_cmd::get_promotion_stats,
            commands::review_cmd::demote_stale_rules,
            commands::review_cmd::reset_retry_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
