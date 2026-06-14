// Mechanized Gate — Enforces structural, taste, golden, and repeated-error rules
// Every agent output is independently verified by this gate after the Review LLM pass.

pub mod rules;
pub mod patterns;
pub mod scoring;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub score: u32,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub category: ViolationCategory,
    pub message: String,
    pub tool_hint: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationCategory {
    Structural,
    Taste,
    Golden,
    Repeated,
}

impl GateResult {
    pub fn pass() -> Self {
        Self { passed: true, score: 100, violations: vec![] }
    }

    pub fn fail(score: u32, violations: Vec<Violation>) -> Self {
        Self { passed: score >= 80, score, violations }
    }
}
