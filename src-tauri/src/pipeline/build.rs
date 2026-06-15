use crate::AppState;
use crate::commands::tools::{ToolRequest, ToolResult, GateViolationInfo};

pub struct BuildAgent;

impl BuildAgent {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute_tool_with_retry(
        state: &AppState,
        tool_req: ToolRequest,
        max_retries: u8,
    ) -> ToolResult {
        let mut last_violations: Vec<GateViolationInfo> = vec![];

        for attempt in 0..=max_retries {
            log::info!("BuildAgent: tool={}, attempt={}/{}", tool_req.tool, attempt + 1, max_retries + 1);

            let mut args = tool_req.args.clone();
            if attempt > 0 && !last_violations.is_empty() {
                let feedback: Vec<String> = last_violations.iter()
                    .map(|v| format!("Gate violation: [{}] {} Hint: {}", v.category, v.message, v.tool_hint.as_deref().unwrap_or("fix manually")))
                    .collect();
                if let Some(obj) = args.as_object_mut() {
                    obj.insert("_gate_feedback".into(), serde_json::Value::String(feedback.join("\n")));
                }
            }

            let req = ToolRequest { tool: tool_req.tool.clone(), args: args.clone() };

            let result = match crate::commands::tools::execute_tool_inner(state, req).await {
                Ok(r) => r,
                Err(e) => {
                    log::error!("BuildAgent: tool={} error on attempt {}: {}", tool_req.tool, attempt + 1, e);
                    if attempt < max_retries {
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("_error_feedback".into(), serde_json::Value::String(format!("Previous attempt failed: {}", e)));
                        }
                        continue;
                    }
                    return ToolResult {
                        success: false, output: String::new(), error: Some(e), gate_result: None,
                    };
                }
            };

            match &result {
                ToolResult { success: true, gate_result: Some(g), .. } if g.passed => {
                    log::info!("BuildAgent: tool={} passed Gate on attempt {}", tool_req.tool, attempt + 1);
                    for v in &g.violations {
                        let mut db = state.rules_db.lock().unwrap();
                        let lang = state.detected_language.lock().unwrap().clone();
                        if let Some(pattern) = v.message.rsplit(": ").next() {
                            db.promote_or_increment(&lang, &v.category.to_lowercase(), pattern, &v.message, "error");
                        }
                    }
                    return result;
                }
                ToolResult { success: true, gate_result: Some(g), .. } => {
                    last_violations = g.violations.clone();
                    log::warn!("BuildAgent: tool={} failed Gate on attempt {} (score={})", tool_req.tool, attempt + 1, g.score);
                }
                ToolResult { success: false, error: Some(e), .. } => {
                    log::error!("BuildAgent: tool={} failed on attempt {}: {}", tool_req.tool, attempt + 1, e);
                    if attempt < max_retries {
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("_error_feedback".into(), serde_json::Value::String(format!("Previous attempt failed: {}", e)));
                        }
                        continue;
                    }
                    return result;
                }
                _ => return result,
            }
        }

        ToolResult {
            success: true,
            output: format!("Written with Gate violations after {} retries", max_retries),
            error: Some("Gate retry limit reached".into()),
            gate_result: None,
        }
    }
}
