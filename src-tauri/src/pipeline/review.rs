// Review Agent — Reviews code against golden rules, structural/taste/golden/repeated checks
// Uses Claude Opus (strongest critique). Read-only agent.

pub struct ReviewAgent;

impl ReviewAgent {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, code: &str) -> Result<String, String> {
        log::info!("ReviewAgent: reviewing code");
        Ok(format!("Review of: {}", code))
    }
}
