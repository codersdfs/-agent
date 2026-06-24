// MCP Client — JSON-RPC transport for Model Context Protocol
// Discovers and invokes skills via the MCP registry.

pub mod transport;
pub mod skills;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub method: String,
    pub params: Option<HashMap<String, serde_json::Value>>,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub endpoint: String,
}

impl Skill {
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read file: {e}"))?;

        let skill: Skill =
            serde_json::from_str(&content).map_err(|e| format!("invalid JSON: {e}"))?;

        if skill.name.is_empty() {
            return Err("skill name is empty".into());
        }
        if skill.endpoint.is_empty() {
            return Err("skill endpoint is empty".into());
        }

        Ok(skill)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_from_valid_json() {
        let json = r#"{"name":"test","description":"A test","endpoint":"http://localhost"}"#;
        let skill: Skill = serde_json::from_str(json).unwrap();
        assert_eq!(skill.name, "test");
        assert_eq!(skill.endpoint, "http://localhost");
    }

    #[test]
    fn skill_from_file_missing() {
        let path = std::path::Path::new("/nonexistent/file.mcp.json");
        let result = Skill::from_file(path);
        assert!(result.is_err());
    }
}
