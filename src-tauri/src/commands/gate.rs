use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;
use crate::commands::tools::GateCheckResult;
use crate::commands::tools::GateViolationInfo;

#[derive(Debug, Serialize, Deserialize)]
pub struct GateCheckRequest {
    pub content: String,
    pub context: String,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RuleEntry {
    pub pattern: String,
    pub severity: String,
    pub message: String,
    pub promoted: bool,
    pub frequency: u32,
}

#[tauri::command]
pub async fn check_gate(
    state: State<'_, AppState>,
    request: GateCheckRequest,
) -> Result<GateCheckResult, String> {
    log::info!("check_gate: content_len={}, context={:?}", request.content.len(), request.context.chars().take(50).collect::<String>());

    let db = state.rules_db.lock().unwrap();
    let lang = if let Some(l) = &request.language {
        harness::Language::Other(l.clone())
    } else {
        state.detected_language.lock().unwrap().clone()
    };

    let violations = db.check_content(&request.content, &lang);

    if violations.is_empty() {
        return Ok(GateCheckResult { passed: true, score: 100, violations: vec![] });
    }

    let gate_result = harness::scoring::calculate_score(&violations);

    Ok(GateCheckResult {
        passed: gate_result.passed,
        score: gate_result.score,
        violations: gate_result.violations.iter().map(|v| GateViolationInfo {
            category: format!("{:?}", v.category),
            message: v.message.clone(),
            tool_hint: v.tool_hint.clone(),
            line: v.line,
        }).collect(),
    })
}

#[tauri::command]
pub async fn get_rules(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.rules_db.lock().unwrap();
    let lang = state.detected_language.lock().unwrap().clone();
    let group = db.load_for_language(&lang);
    let mut entries = vec![];
    for (rule, _cat) in group.all_rules() {
        if rule.promoted || rule.frequency >= 3 {
            entries.push(format!("[promoted] {} (freq={}): {}", rule.severity, rule.frequency, rule.message));
        }
    }
    if entries.is_empty() {
        return Ok(vec!["No promoted rules yet".into()]);
    }
    Ok(entries)
}

#[tauri::command]
pub async fn reset_rules(
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mut db = state.rules_db.lock().unwrap();
    *db = harness::rules::RulesDatabase::new();
    Ok("Rules database reset to defaults".into())
}

#[tauri::command]
pub async fn set_review_mode(
    state: State<'_, AppState>,
    mode: String,
) -> Result<String, String> {
    let mut config = state.review_config.lock().unwrap();
    config.mode = match mode.as_str() {
        "off" => crate::pipeline::ReviewMode::Off,
        "summary" => crate::pipeline::ReviewMode::Summary,
        "live" => crate::pipeline::ReviewMode::Live,
        other => return Err(format!("Unknown review mode: {}. Use off, summary, or live", other)),
    };
    Ok(format!("Review mode set to {}", mode))
}
