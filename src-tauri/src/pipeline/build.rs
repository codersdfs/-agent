use crate::AppState;
use crate::commands::tools::{ToolRequest, ToolResult, GateViolationInfo};
use crate::pipeline::plan::StructuredPlan;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSessionEntry {
    pub step_index: usize,
    pub tool: String,
    pub args: serde_json::Value,
    pub success: bool,
    pub output_preview: String,
    pub error: Option<String>,
    pub gate_passed: Option<bool>,
    pub gate_score: Option<u32>,
    pub duration_ms: u64,
    pub retries: u8,
    pub timestamp_start: String,
    pub timestamp_end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub tool: String,
    pub args: serde_json::Value,
    pub reason: String,
    pub step_id: u32,
    pub step_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProgress {
    pub total_steps: usize,
    pub completed_steps: usize,
    pub current_step: usize,
    pub status: String,
    pub total_retries: u32,
}

pub struct BuildAgent;

impl BuildAgent {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute_plan(
        &self,
        state: &AppState,
        plan: &StructuredPlan,
    ) -> Result<Vec<BuildSessionEntry>, String> {
        let mut session: Vec<BuildSessionEntry> = vec![];

        log::info!("BuildAgent: executing plan with {} steps", plan.steps.len());
        println!("[build] executing plan with {} steps", plan.steps.len());

        {
            let mut p = state.pipeline.lock().await;
            p.status = crate::pipeline::PipelineStatus::Building;
            p.current_step_index = 0;
        }

        for (step_idx, step) in plan.steps.iter().enumerate() {
            log::info!("BuildAgent: step {}/{}: {}", step_idx + 1, plan.steps.len(), step.description);
            println!("[build] step {}/{}: {}", step_idx + 1, plan.steps.len(), step.description);

            {
                let mut p = state.pipeline.lock().await;
                p.current_step_index = step_idx;
                p.status = crate::pipeline::PipelineStatus::Building;
            }

            let needs_permission = matches!(step.action.as_str(), "create" | "modify" | "delete");
            if needs_permission {
                let perm_req = PermissionRequest {
                    id: uuid::Uuid::new_v4().to_string(),
                    tool: match step.action.as_str() {
                        "delete" => "bash".into(),
                        _ => "write".into(),
                    },
                    args: serde_json::json!({
                        "filePath": step.file_path,
                        "description": step.description,
                    }),
                    reason: format!("Step #{}: {} — {}", step.id, step.action, step.description),
                    step_id: step.id,
                    step_description: step.description.clone(),
                };

                println!("[build] permission: {} (y/n)", perm_req.reason);
                let approved = Self::wait_for_permission(state).await;
                if !approved {
                    session.push(BuildSessionEntry {
                        step_index: step_idx,
                        tool: perm_req.tool,
                        args: perm_req.args,
                        success: false,
                        output_preview: String::new(),
                        error: Some("Permission denied by user".into()),
                        gate_passed: None,
                        gate_score: None,
                        duration_ms: 0,
                        retries: 0,
                        timestamp_start: chrono::Utc::now().to_rfc3339(),
                        timestamp_end: chrono::Utc::now().to_rfc3339(),
                    });
                    println!("[build] step {}: denied", step_idx);
                    continue;
                }
            }

            let tool_req = Self::step_to_tool_request(state, step).await;
            let start = Instant::now();
            let start_ts = chrono::Utc::now().to_rfc3339();

            let result = Self::execute_tool_with_retry(state, tool_req.clone(), 3).await;
            let duration_ms = start.elapsed().as_millis() as u64;
            let end_ts = chrono::Utc::now().to_rfc3339();

            let entry = BuildSessionEntry {
                step_index: step_idx,
                tool: tool_req.tool,
                args: tool_req.args,
                success: result.success,
                output_preview: result.output.chars().take(200).collect(),
                error: result.error.clone(),
                gate_passed: result.gate_result.as_ref().map(|g| g.passed),
                gate_score: result.gate_result.as_ref().map(|g| g.score),
                duration_ms,
                retries: 0,
                timestamp_start: start_ts,
                timestamp_end: end_ts,
            };

            let status = if result.success { "completed" } else { "failed" };
            println!("[build] step {}: {} (gate {:?} score {:?} {}ms)", step_idx, status, entry.gate_passed, entry.gate_score, duration_ms);
            session.push(entry);
        }

        {
            let mut p = state.pipeline.lock().await;
            p.status = crate::pipeline::PipelineStatus::Idle;
            p.build_output = Some(format!("Completed {} steps", session.len()));
        }
        {
            let mut log = state.session_log.lock().unwrap();
            *log = session.clone();
        }

        let completed = session.iter().filter(|e| e.success).count();
        let total_ms: u64 = session.iter().map(|e| e.duration_ms).sum();
        println!("[build] complete: {}/{} steps, {}ms total", completed, plan.steps.len(), total_ms);

        Ok(session)
    }

    async fn execute_tool_with_retry(
        state: &AppState,
        tool_req: ToolRequest,
        max_retries: u8,
    ) -> ToolResult {
        let mut last_violations: Vec<GateViolationInfo> = vec![];

        for attempt in 0..=max_retries {
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
            println!("[build] tool={} attempt={}/{}", req.tool, attempt + 1, max_retries + 1);

            let result = match crate::commands::tools::execute_tool_inner(state, req).await {
                Ok(r) => r,
                Err(e) => {
                    if attempt < max_retries {
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("_error_feedback".into(), serde_json::Value::String(format!("Previous attempt failed: {}", e)));
                        }
                        continue;
                    }
                    return ToolResult { success: false, output: String::new(), error: Some(e), gate_result: None };
                }
            };

            match result {
                ToolResult { success: true, gate_result: Some(ref g), .. } if g.passed => {
                    for v in &g.violations {
                        let mut db = state.rules_db.lock().unwrap();
                        let lang = state.detected_language.lock().unwrap().clone();
                        if let Some(pattern) = v.message.rsplit(": ").next() {
                            db.promote_or_increment(&lang, &v.category.to_lowercase(), pattern, &v.message, "error");
                        }
                    }
                    println!("[build] tool={} passed gate (score={}) on attempt {}", tool_req.tool, g.score, attempt + 1);
                    return result;
                }
                ToolResult { success: true, gate_result: Some(ref g), .. } => {
                    last_violations = g.violations.clone();
                    println!("[build] tool={} failed gate (score={}) on attempt {}", tool_req.tool, g.score, attempt + 1);
                }
                ToolResult { success: false, ref error, .. } => {
                    if let Some(ref e) = error {
                        if attempt < max_retries {
                            if let Some(obj) = args.as_object_mut() {
                                obj.insert("_error_feedback".into(), serde_json::Value::String(format!("Previous attempt failed: {}", e)));
                            }
                            continue;
                        }
                        println!("[build] tool={} failed after {} attempts: {}", tool_req.tool, attempt + 1, e);
                        return result;
                    }
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

    async fn step_to_tool_request(_state: &AppState, step: &crate::pipeline::plan::PlanStep) -> ToolRequest {
        match step.action.as_str() {
            "create" | "modify" => ToolRequest {
                tool: "write".into(),
                args: serde_json::json!({
                    "filePath": step.file_path,
                    "content": "",
                    "_step_description": step.description,
                }),
            },
            "delete" => ToolRequest {
                tool: "bash".into(),
                args: serde_json::json!({
                    "command": format!("Remove-Item -LiteralPath \"{}\"", step.file_path.as_deref().unwrap_or("")),
                }),
            },
            _ => ToolRequest {
                tool: "bash".into(),
                args: serde_json::json!({ "command": step.description }),
            },
        }
    }

    async fn wait_for_permission(state: &AppState) -> bool {
        if state.build_config.lock().unwrap().auto_approve {
            return true;
        }
        use std::io::{stdin, stdout, Write};
        print!("> Allow? (y/n): ");
        let _ = stdout().flush();
        let mut line = String::new();
        match stdin().read_line(&mut line) {
            Ok(_) => {
                let trimmed = line.trim().to_lowercase();
                trimmed == "y" || trimmed == "yes"
            }
            Err(_) => false,
        }
    }
}
