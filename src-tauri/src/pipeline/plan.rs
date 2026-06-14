// Plan Agent — Reads task, produces structured plan (.otable format)
// Read-only agent (no write/edit/bash tools)

pub struct PlanAgent;

impl PlanAgent {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, task: &str) -> Result<String, String> {
        log::info!("PlanAgent: planning task={:?}", task.chars().take(80).collect::<String>());
        Ok(format!("Plan for: {}", task))
    }
}
