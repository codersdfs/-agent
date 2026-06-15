use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::commands::tools::GateCheckResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentType {
    Plan,
    Build,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    Idle,
    Planning,
    Building,
    Reviewing,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewMode {
    Off,
    Summary,
    Live,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    pub mode: ReviewMode,
    pub max_retries: u8,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self { mode: ReviewMode::Summary, max_retries: 3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool: String,
    pub args: serde_json::Value,
    pub result: Option<GateCheckResult>,
    pub retry_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub task_id: String,
    pub status: PipelineStatus,
    pub retry_count: u8,
    pub max_retries: u8,
    pub current_score: u32,
    pub pass_threshold: u32,
    pub tools_called: Vec<ToolCallRecord>,
    pub gate_violations: Vec<crate::commands::tools::GateViolationInfo>,
    pub plan: Option<String>,
    pub build_output: Option<String>,
    pub review_output: Option<String>,
}

impl PipelineState {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            status: PipelineStatus::Idle,
            retry_count: 0,
            max_retries: 3,
            current_score: 0,
            pass_threshold: 80,
            tools_called: vec![],
            gate_violations: vec![],
            plan: None,
            build_output: None,
            review_output: None,
        }
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

pub type SharedPipelineState = Arc<Mutex<PipelineState>>;
