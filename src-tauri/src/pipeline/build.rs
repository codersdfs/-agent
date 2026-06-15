use crate::AppState;
use crate::commands::tools::{ToolRequest, ToolResult, GateViolationInfo};
use crate::pipeline::plan::StructuredPlan;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tauri::Emitter;

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

    /// Execute the full build pipeline from a plan.
    /// Orchestrates tool calls with permissions, Gate retry loop, and session logging.
    pub async fn execute_plan(
        &self,
        state: &AppState,
        plan: &StructuredPlan,
        app_handle: &tauri::AppHandle,
    ) -> Result<Vec<BuildSessionEntry>, String> {
        let mut session: Vec<BuildSessionEntry> = vec![];

        log::info!("BuildAgent: executing plan with {} steps", plan.steps.len());

        // Set pipeline status
        {
            let mut p = state.pipeline.lock().await;
            p.status = crate::pipeline::PipelineStatus::Building;
            p.current_step_index = 0;
        }

        for (step_idx, step) in plan.steps.iter().enumerate() {
            log::info!("BuildAgent: executing step {}/{}: {}", step_idx + 1, plan.steps.len(), step.description);

            // Update pipeline state
            {
                let mut p = state.pipeline.lock().await;
                p.current_step_index = step_idx;
                p.status = crate::pipeline::PipelineStatus::Building;
            }

            // Emit build step start event
            let _ = app_handle.emit("build-step-start", serde_json::json!({
                "step_index": step_idx,
                "step_id": step.id,
                "description": step.description,
                "action": step.action,
                "file_path": step.file_path,
            }));

            // Check if this step needs permission (write/edit/bash)
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

                let _ = app_handle.emit("build-permission-request", &perm_req);

                // Wait for permission response (frontend sends via respond_permission command)
                let approved = Self::wait_for_permission(&state, &perm_req.id).await;
                if !approved {
                    let entry = BuildSessionEntry {
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
                    };
                    session.push(entry);
                    let _ = app_handle.emit("build-step-end", serde_json::json!({
                        "step_index": step_idx,
                        "status": "denied",
                    }));
                    continue;
                }
            }

            // Construct the tool request from the plan step
            let tool_req = Self::step_to_tool_request(state, step).await;
            let start = Instant::now();
            let start_ts = chrono::Utc::now().to_rfc3339();

            // Execute with Gate retry loop
            let result = Self::execute_tool_with_retry(state, tool_req.clone(), app_handle, 3).await;
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
            let _ = app_handle.emit("build-step-end", serde_json::json!({
                "step_index": step_idx,
                "status": status,
                "gate_passed": entry.gate_passed,
                "gate_score": entry.gate_score,
                "duration_ms": duration_ms,
            }));

            session.push(entry);
        }

        // Update pipeline state and session log
        {
            let mut p = state.pipeline.lock().await;
            p.status = crate::pipeline::PipelineStatus::Idle;
            p.build_output = Some(format!("Completed {} steps", session.len()));
        }
        {
            let mut log = state.session_log.lock().unwrap();
            *log = session.clone();
        }

        let _ = app_handle.emit("build-complete", serde_json::json!({
            "total_steps": plan.steps.len(),
            "completed_steps": session.iter().filter(|e| e.success).count(),
            "duration_ms_total": session.iter().map(|e| e.duration_ms).sum::<u64>(),
        }));

        Ok(session)
    }

    /// Execute a single tool call with Gate retry loop.
    async fn execute_tool_with_retry(
        state: &AppState,
        tool_req: ToolRequest,
        app_handle: &tauri::AppHandle,
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
            let _ = app_handle.emit("build-tool-exec", serde_json::json!({
                "tool": req.tool,
                "attempt": attempt + 1,
                "max_retries": max_retries + 1,
            }));

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
                    let _ = app_handle.emit("build-tool-result", serde_json::json!({
                        "tool": tool_req.tool,
                        "success": true,
                        "gate_passed": true,
                        "attempt": attempt + 1,
                    }));
                    return result;
                }
                ToolResult { success: true, gate_result: Some(g), .. } => {
                    last_violations = g.violations.clone();
                    log::warn!("BuildAgent: tool={} failed Gate on attempt {} (score={})", tool_req.tool, attempt + 1, g.score);
                    let _ = app_handle.emit("build-tool-result", serde_json::json!({
                        "tool": tool_req.tool,
                        "success": true,
                        "gate_passed": false,
                        "gate_score": g.score,
                        "violations": g.violations,
                        "attempt": attempt + 1,
                    }));
                }
                ToolResult { success: false, error: Some(e), .. } => {
                    log::error!("BuildAgent: tool={} failed on attempt {}: {}", tool_req.tool, attempt + 1, e);
                    if attempt < max_retries {
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("_error_feedback".into(), serde_json::Value::String(format!("Previous attempt failed: {}", e)));
                        }
                        continue;
                    }
                    let _ = app_handle.emit("build-tool-result", serde_json::json!({
                        "tool": tool_req.tool,
                        "success": false,
                        "error": e,
                        "attempt": attempt + 1,
                    }));
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

    /// Convert a plan step into a tool request.
    async fn step_to_tool_request(_state: &AppState, step: &crate::pipeline::plan::PlanStep) -> ToolRequest {
        match step.action.as_str() {
            "create" | "modify" => {
                // For create/modify, we need to read the file first to construct a write/edit
                let tool = "write";
                let args = serde_json::json!({
                    "filePath": step.file_path,
                    "content": "", // LLM will fill this
                    "_step_description": step.description,
                });
                ToolRequest {
                    tool: tool.into(),
                    args,
                }
            }
            "delete" => {
                ToolRequest {
                    tool: "bash".into(),
                    args: serde_json::json!({
                        "command": format!("Remove-Item -LiteralPath \"{}\"", step.file_path.as_deref().unwrap_or("")),
                    }),
                }
            }
            "refactor" | "test" => {
                ToolRequest {
                    tool: "bash".into(),
                    args: serde_json::json!({
                        "command": step.description,
                    }),
                }
            }
            _ => {
                ToolRequest {
                    tool: "bash".into(),
                    args: serde_json::json!({
                        "command": step.description,
                    }),
                }
            }
        }
    }

    /// Wait for user to approve/deny a permission request.
    async fn wait_for_permission(state: &AppState, request_id: &str) -> bool {
        // Check if auto-approve is configured
        let auto_approve = state.build_config.lock().unwrap().auto_approve;
        if auto_approve {
            return true;
        }

        // Register the pending permission
        let key = request_id.to_string();
        {
            let mut pending = state.pending_permissions.lock().unwrap();
            pending.insert(key.clone());
        }

        // Wait up to 120 seconds for a response
        let mut elapsed = 0u64;
        while elapsed < 120 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            elapsed += 1;

            let pending = state.pending_permissions.lock().unwrap();
            if !pending.contains(&key) {
                // Permission was responded to (removed from set)
                return state.permission_results.lock().unwrap().get(&key).copied().unwrap_or(false);
            }
        }

        // Timeout — deny
        false
    }
}
