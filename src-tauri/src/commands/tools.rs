use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;
use std::path::PathBuf;
use regex::Regex;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolRequest {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateViolationInfo {
    pub category: String,
    pub message: String,
    pub tool_hint: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub gate_result: Option<GateCheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateCheckResult {
    pub passed: bool,
    pub score: u32,
    pub violations: Vec<GateViolationInfo>,
}

impl GateCheckResult {
    pub fn from_harness(g: &harness::GateResult) -> Self {
        Self {
            passed: g.passed,
            score: g.score,
            violations: g.violations.iter().map(|v| GateViolationInfo {
                category: format!("{:?}", v.category),
                message: v.message.clone(),
                tool_hint: v.tool_hint.clone(),
                line: v.line,
            }).collect(),
        }
    }
}

async fn run_gate(state: &AppState, content: &str) -> GateCheckResult {
    let db = state.rules_db.lock().unwrap();
    let lang = state.detected_language.lock().unwrap().clone();
    let violations = db.check_content(content, &lang);

    if violations.is_empty() {
        return GateCheckResult { passed: true, score: 100, violations: vec![] };
    }

    let gate_result = harness::scoring::calculate_score(&violations);
    GateCheckResult::from_harness(&gate_result)
}

async fn read_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))
}

async fn write_file(path: &str, content: &str) -> Result<(), String> {
    if let Some(parent) = PathBuf::from(path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
    }
    std::fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path, e))
}

async fn edit_file(path: &str, old_string: &str, new_string: &str, replace_all: bool) -> Result<(), String> {
    let content = read_file(path).await?;
    let new_content = if replace_all {
        content.replace(old_string, new_string)
    } else {
        match content.find(old_string) {
            Some(_) => content.replacen(old_string, new_string, 1),
            None => return Err(format!("oldString not found in {}", path)),
        }
    };
    if new_content == content {
        return Err(format!("No changes made to {}", path));
    }
    write_file(path, &new_content).await
}

fn run_bash(command: &str) -> Result<String, String> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", command])
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let combined = if stderr.is_empty() { stdout.clone() } else { format!("{}\n{}", stdout, stderr) };
        return Err(combined.trim().to_string());
    }

    Ok(stdout.trim().to_string())
}

fn run_grep(pattern: &str, path: &str, include: Option<&str>) -> Result<String, String> {
    let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
    let search_dir = PathBuf::from(path);
    if !search_dir.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let mut results = vec![];
    let walker = walkdir::WalkDir::new(&search_dir).max_depth(10);

    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() { continue; }
        let file_path = entry.path();

        if let Some(ext) = include {
            if !file_path.to_string_lossy().ends_with(ext.trim_start_matches('*')) {
                continue;
            }
        }

        if let Ok(content) = std::fs::read_to_string(file_path) {
            for (i, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(format!("{}:{}: {}", file_path.display(), i + 1, line.trim()));
                }
            }
        }
    }

    if results.is_empty() {
        return Ok("No matches found".into());
    }
    Ok(results.join("\n"))
}

fn run_glob(pattern: &str, path: Option<&str>) -> Result<String, String> {
    let base = path.unwrap_or(".");
    let full_pattern = format!("{}/{}", base.trim_end_matches('/').trim_end_matches('\\'), pattern);
    let glob_pattern = glob::glob(&full_pattern)
        .map_err(|e| format!("Invalid glob pattern: {}", e))?;

    let paths: Vec<String> = glob_pattern
        .filter_map(|e| e.ok())
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    if paths.is_empty() {
        return Ok("No matches found".into());
    }
    Ok(paths.join("\n"))
}

/// Inner fn — no Tauri state wrapper, used by BuildAgent retry loop.
pub async fn execute_tool_inner(
    state: &AppState,
    request: ToolRequest,
) -> Result<ToolResult, String> {
    log::info!("execute_tool_inner: tool={}", request.tool);

    let make_err = |msg: &str| ToolResult {
        success: false, output: String::new(), error: Some(msg.to_string()), gate_result: None,
    };

    let result = match request.tool.as_str() {
        "read" => {
            let path = request.args.get("filePath")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: filePath".to_string())?;
            let content = read_file(path).await?;
            let gate = run_gate(state, &content).await;
            ToolResult { success: true, output: content, error: None, gate_result: Some(gate) }
        }
        "write" => {
            let path = request.args.get("filePath")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: filePath".to_string())?;
            let content = request.args.get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: content".to_string())?;

            let gate = run_gate(state, content).await;
            write_file(path, content).await?;

            if !gate.passed {
                let violation_summary: Vec<String> = gate.violations.iter()
                    .map(|v| format!("[{}] {}", v.category, v.message))
                    .collect();
                ToolResult {
                    success: true,
                    output: format!("Written. Gate violations ({}):\n{}", gate.score, violation_summary.join("\n")),
                    error: None,
                    gate_result: Some(gate),
                }
            } else {
                ToolResult { success: true, output: "Written successfully".into(), error: None, gate_result: Some(gate) }
            }
        }
        "edit" => {
            let path = request.args.get("filePath")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: filePath".to_string())?;
            let old_string = request.args.get("oldString")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: oldString".to_string())?;
            let new_string = request.args.get("newString")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: newString".to_string())?;
            let replace_all = request.args.get("replaceAll")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            edit_file(path, old_string, new_string, replace_all).await?;

            let updated = read_file(path).await?;
            let gate = run_gate(state, &updated).await;

            ToolResult { success: true, output: "Edited successfully".into(), error: None, gate_result: Some(gate) }
        }
        "bash" => {
            let command = request.args.get("command")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: command".to_string())?;
            match run_bash(command) {
                Ok(output) => ToolResult { success: true, output, error: None, gate_result: None },
                Err(e) => make_err(&e),
            }
        }
        "grep" => {
            let pattern = request.args.get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: pattern".to_string())?;
            let path = request.args.get("path").and_then(|v| v.as_str());
            let include = request.args.get("include").and_then(|v| v.as_str());
            match run_grep(pattern, path.unwrap_or("."), include) {
                Ok(output) => ToolResult { success: true, output, error: None, gate_result: None },
                Err(e) => make_err(&e),
            }
        }
        "glob" => {
            let pattern = request.args.get("pattern")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing argument: pattern".to_string())?;
            let path = request.args.get("path").and_then(|v| v.as_str());
            match run_glob(pattern, path) {
                Ok(output) => ToolResult { success: true, output, error: None, gate_result: None },
                Err(e) => make_err(&e),
            }
        }
        _ => {
            make_err(&format!("Unknown tool: {}", request.tool))
        }
    };

    // Auto-promote violations from write/edit
    if matches!(request.tool.as_str(), "write" | "edit") {
        if let Some(ref g) = result.gate_result {
            if !g.passed {
                let mut db = state.rules_db.lock().unwrap();
                let lang = state.detected_language.lock().unwrap().clone();
                for v in &g.violations {
                    let cat = v.category.to_lowercase();
                    if let Some(pattern) = v.message.rsplit(": ").next() {
                        db.promote_or_increment(&lang, &cat, pattern, &v.message, "error");
                    }
                }
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn execute_tool(
    state: State<'_, AppState>,
    request: ToolRequest,
) -> Result<ToolResult, String> {
    execute_tool_inner(&state, request).await
}

#[tauri::command]
pub async fn list_tools() -> Result<Vec<String>, String> {
    Ok(vec![
        "read".into(),
        "write".into(),
        "edit".into(),
        "bash".into(),
        "grep".into(),
        "glob".into(),
    ])
}
