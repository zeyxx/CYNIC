#[derive(Debug, Deserialize, JsonSchema)]
pub struct InfraProvisionParams {
    /// The target node or service to provision (e.g. kernel_primary, storage_primary)
    pub target: String,
}

#[tool(
    name = "infra_provision",
    description = "Provisions infrastructure targets (like storage_primary or kernel_primary) through the authenticated kernel control-plane. If MCP is unavailable, report blocked_disconnected; never fall back to local execution."
)]
async fn infra_provision(
    &self,
    params: Parameters<InfraProvisionParams>,
) -> Result<CallToolResult, McpError> {
    self.require_auth()?;
    self.rate_limit.check_other()?;

    let req = params.0;
    let response = self
        .client
        .post(format!("{}/lifecycle/provision", self.rest_addr))
        .json(&req)
        .send()
        .await
        .map_err(|e| McpError::InternalError(e.to_string()))?;

    let status = response.status();
    let body_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read response body".to_string());

    if status.is_success() {
        Ok(CallToolResult {
            content: vec![mcp_core::content::Content::Text(
                mcp_core::content::TextContent { text: body_text },
            )],
            is_error: Some(false),
        })
    } else {
        Err(McpError::InternalError(format!(
            "Failed to provision infrastructure: {} - {}",
            status, body_text
        )))
    }
}
