use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    #[serde(default)]
    pub permission_mode: Option<String>,
}

impl CliConfig {
    pub fn to_provider_config(&self) -> providers::ProviderConfig {
        let mut cfg = providers::ProviderConfig::default();
        if let Some(ref provider) = self.provider {
            cfg.kind = providers::ProviderKind::from_str(provider);
        }
        if let Some(ref model) = self.model {
            cfg.model = model.clone();
        }
        cfg.base_url = self.base_url.clone();
        cfg.api_key = load_api_key();
        cfg
    }

    pub fn from_provider_config(cfg: &providers::ProviderConfig) -> Self {
        let existing = Self::load_existing();
        Self {
            provider: Some(cfg.kind.to_string()),
            model: Some(cfg.model.clone()),
            base_url: cfg.base_url.clone(),
            permission_mode: existing.permission_mode,
        }
    }

    fn load_existing() -> Self {
        let path = config_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "omega", "omega-agent") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        PathBuf::from(".omega")
    }
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn env_path() -> PathBuf {
    config_dir().join(".env")
}

pub fn load_config() -> CliConfig {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_default()
            }
            Err(_) => CliConfig::default(),
        }
    } else {
        CliConfig::default()
    }
}

pub fn save_config(config: &CliConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| format!("Failed to write config: {}", e))
}

pub fn load_api_key() -> Option<String> {
    let path = env_path();
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("OMEGA_API_KEY=") {
            let val = line.strip_prefix("OMEGA_API_KEY=").unwrap_or("");
            if val.is_empty() {
                return None;
            }
            return Some(val.to_string());
        }
    }
    None
}

pub fn save_api_key(key: &str) -> Result<(), String> {
    let path = env_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    std::fs::write(&path, format!("OMEGA_API_KEY={}", key))
        .map_err(|e| format!("Failed to write .env: {}", e))
}

pub fn load_provider_config() -> providers::ProviderConfig {
    migrate_base_urls();
    load_config().to_provider_config()
}

fn migrate_base_urls() {
    let mut cfg = load_config();
    let old_urls = [
        ("https://api.openai.com", "https://api.openai.com/v1"),
        ("https://api.x.ai", "https://api.x.ai/v1"),
        ("https://api.cerebras.ai", "https://api.cerebras.ai/v1"),
        ("https://api.groq.com", "https://api.groq.com/openai/v1"),
        ("https://api.moonshot.cn", "https://api.moonshot.cn/v1"),
        ("https://api.minimax.chat", "https://api.minimax.chat/v1"),
        ("https://openrouter.ai", "https://openrouter.ai/api/v1"),
        ("https://YOUR_RESOURCE.openai.azure.com", "https://YOUR_RESOURCE.openai.azure.com/v1"),
        ("https://api-inference.huggingface.co", "https://api-inference.huggingface.co/v1"),
    ];
    if let Some(ref url) = cfg.base_url {
        for (old, new) in &old_urls {
            if url == old {
                cfg.base_url = Some(new.to_string());
                let _ = save_config(&cfg);
                return;
            }
        }
    }
}


