use crate::{McpRequest, McpResponse};

pub struct JsonRpcTransport {
    endpoint: String,
    client: reqwest::Client,
}

impl JsonRpcTransport {
    pub fn new(endpoint: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            endpoint: endpoint.to_string(),
            client,
        }
    }

    pub async fn send(&self, request: McpRequest) -> Result<McpResponse, String> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": request.id,
            "method": request.method,
            "params": request.params
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("MCP transport error: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("MCP HTTP {status}: {text}"));
        }

        let mcp_resp: McpResponse = resp
            .json()
            .await
            .map_err(|e| format!("MCP parse error: {e}"))?;

        Ok(mcp_resp)
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}
