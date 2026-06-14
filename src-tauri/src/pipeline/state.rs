use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

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
pub struct PipelineState {
    pub task_id: String,
    pub status: PipelineStatus,
    pub retry_count: u8,
    pub max_retries: u8,
    pub current_score: u32,
    pub pass_threshold: u32,
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
        }
    }
}

pub type SharedPipelineState = Arc<Mutex<PipelineState>>;
