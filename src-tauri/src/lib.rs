pub mod commands;
pub mod pipeline;

use pipeline::build::BuildSessionEntry;
use std::collections::{HashMap, HashSet};
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
