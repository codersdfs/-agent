pub mod openai;
pub mod anthropic;
pub mod google;
pub mod mistral;
pub mod local;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum ProviderKind {
    Anthropic,
    OpenAI,
    Google,
    Mistral,
    XAI,
    Cerebras,
    Azure,
    Bedrock,
    HuggingFace,
    Groq,
    Kimi,
    MiniMax,
    OpenRouter,
    Local,
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Mistral => "mistral",
            Self::XAI => "xai",
            Self::Cerebras => "cerebras",
            Self::Azure => "azure",
            Self::Bedrock => "bedrock",
            Self::HuggingFace => "huggingface",
            Self::Groq => "groq",
            Self::Kimi => "kimi",
            Self::MiniMax => "minimax",
            Self::OpenRouter => "openrouter",
            Self::Local => "local",
        };
        write!(f, "{}", s)
    }
}

impl ProviderKind {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "anthropic" => Self::Anthropic,
            "openai" => Self::OpenAI,
            "google" => Self::Google,
            "mistral" => Self::Mistral,
            "xai" => Self::XAI,
            "cerebras" => Self::Cerebras,
            "azure" => Self::Azure,
            "bedrock" => Self::Bedrock,
            "huggingface" => Self::HuggingFace,
            "groq" => Self::Groq,
            "kimi" => Self::Kimi,
            "minimax" => Self::MiniMax,
            "openrouter" => Self::OpenRouter,
            "local" => Self::Local,
            _ => Self::OpenAI,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Anthropic, Self::OpenAI, Self::Google, Self::Mistral,
            Self::XAI, Self::Cerebras, Self::Azure, Self::Bedrock,
            Self::HuggingFace, Self::Groq, Self::Kimi, Self::MiniMax,
            Self::OpenRouter, Self::Local,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::OpenAI,
            api_key: None,
            base_url: None,
            model: "gpt-4".into(),
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub config: ProviderConfig,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: String,
    pub done: bool,
    pub model: Option<String>,
    pub usage: Option<Usage>,
}

#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, String>;
    async fn chat_stream(&self, request: ChatRequest, tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>) -> Result<(), String>;
}

pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn LlmProvider>, String> {
    let api_key = config.api_key.clone().unwrap_or_default();
    let base_url = config.base_url.clone();

    match config.kind {
        ProviderKind::OpenAI | ProviderKind::XAI | ProviderKind::Cerebras
        | ProviderKind::Groq | ProviderKind::Kimi | ProviderKind::MiniMax
        | ProviderKind::OpenRouter | ProviderKind::Azure | ProviderKind::Bedrock
        | ProviderKind::HuggingFace => {
            let url = base_url.clone().unwrap_or_else(|| match config.kind {
                ProviderKind::OpenAI => "https://api.openai.com/v1".into(),
                ProviderKind::XAI => "https://api.x.ai/v1".into(),
                ProviderKind::Cerebras => "https://api.cerebras.ai/v1".into(),
                ProviderKind::Groq => "https://api.groq.com/openai/v1".into(),
                ProviderKind::Kimi => "https://api.moonshot.cn/v1".into(),
                ProviderKind::MiniMax => "https://api.minimax.chat/v1".into(),
                ProviderKind::OpenRouter => "https://openrouter.ai/api/v1".into(),
                ProviderKind::Azure => "https://YOUR_RESOURCE.openai.azure.com/v1".into(),
                ProviderKind::Bedrock => "https://bedrock-runtime.YOUR_REGION.amazonaws.com".into(),
                ProviderKind::HuggingFace => "https://api-inference.huggingface.co/v1".into(),
                _ => unreachable!(),
            });
            Ok(Box::new(openai::OpenAIProvider::new(api_key, url)))
        }
        ProviderKind::Anthropic => {
            Ok(Box::new(anthropic::AnthropicProvider::new(api_key, base_url)))
        }
        ProviderKind::Google => {
            Ok(Box::new(google::GoogleProvider::new(api_key, base_url)))
        }
        ProviderKind::Mistral => {
            Ok(Box::new(mistral::MistralProvider::new(api_key, base_url)))
        }
        ProviderKind::Local => {
            let url = base_url.unwrap_or_else(|| "http://localhost:1234/v1".into());
            Ok(Box::new(local::LocalProvider::new(url)))
        }
    }
}
