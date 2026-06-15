use crate::commands::tools::GateCheckResult;
use crate::commands::tools::GateViolationInfo;
use crate::AppState;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ReviewAgent;

impl ReviewAgent {
    pub fn new() -> Self {
        Self
    }

    /// Run Gate check (always on, synchronous, fast).
    /// Returns violations and score.
    pub fn gate_check(state: &AppState, content: &str) -> Vec<GateViolationInfo> {
        let db = state.rules_db.lock().unwrap();
        let lang = state.detected_language.lock().unwrap().clone();
        let violations = db.check_content(content, &lang);

        if violations.is_empty() {
            return vec![];
        }

        violations.iter().map(|v| GateViolationInfo {
            category: format!("{:?}", v.category),
            message: v.message.clone(),
            tool_hint: v.tool_hint.clone(),
            line: v.line,
        }).collect()
    }

    /// Run LLM review (togglable). Calls the configured provider for review.
    pub async fn llm_review(
        state: &AppState,
        code: &str,
        context: &str,
    ) -> Result<String, String> {
        let config = state.provider_config.lock().unwrap().clone();
        let review_prompt = format!(
            "You are a Code Review agent. Analyze this code for:\n\
            1. Logic errors and bugs\n\
            2. Missing error handling\n\
            3. Performance issues\n\
            4. Security vulnerabilities\n\
            5. Architectural problems\n\n\
            Context: {}\n\n\
            Code:\n```\n{}\n```\n\n\
            Provide specific, actionable feedback. Include what, where, and how to fix.",
            context, code
        );

        let provider = providers::create_provider(&config)?;
        let messages = vec![
            providers::ChatMessage { role: "user".into(), content: review_prompt },
        ];

        let chat_request = providers::ChatRequest {
            messages,
            config,
            stream: false,
        };

        let response = provider.chat(chat_request).await?;
        Ok(response.content)
    }

    /// Combined review: Gate + LLM (if mode permits).
    /// Returns (gate_violations, llm_review_comment, score, passed).
    pub async fn combined_review(
        state: &AppState,
        code: &str,
        context: &str,
    ) -> (Vec<GateViolationInfo>, Option<String>, u32, bool) {
        let gate_violations = Self::gate_check(state, code);

        // Score gate violations
        let har_violations: Vec<harness::Violation> = gate_violations.iter().map(|v| {
            let cat = match v.category.to_lowercase().as_str() {
                "structural" => harness::ViolationCategory::Structural,
                "taste" => harness::ViolationCategory::Taste,
                "golden" => harness::ViolationCategory::Golden,
                "repeated" => harness::ViolationCategory::Repeated,
                _ => harness::ViolationCategory::Structural,
            };
            harness::Violation {
                category: cat,
                message: v.message.clone(),
                tool_hint: v.tool_hint.clone(),
                line: v.line,
            }
        }).collect();

        let gate_result = harness::scoring::calculate_score(&har_violations);

        let config = state.review_config.lock().unwrap().clone();
        let llm_output = match config.mode {
            crate::pipeline::ReviewMode::Off => None,
            _ => {
                match Self::llm_review(state, code, context).await {
                    Ok(review) => {
                        if review.len() > 50 { Some(review) } else { None }
                    }
                    Err(e) => Some(format!("LLM review failed: {}", e)),
                }
            }
        };

        (gate_violations, llm_output, gate_result.score, gate_result.passed)
    }
}
