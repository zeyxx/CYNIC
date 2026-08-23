impl CynicMcpProxy {
    async fn get(&self, path: &str) -> Result<CallToolResult, McpError> {
        let response = self
            .kernel
            .client()
            .get(self.kernel.url(path))
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("REST kernel unreachable: {error}"), None)
            })?;
        Self::tool_result_from_response("GET", path, response).await
    }

    async fn get_query(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<CallToolResult, McpError> {
        let url = query_url(self.kernel.base_url(), path, params)?;
        let response = self
            .kernel
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("REST kernel unreachable: {error}"), None)
            })?;
        Self::tool_result_from_response("GET", path, response).await
    }

    async fn get_query_json(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, McpError> {
        let url = query_url(self.kernel.base_url(), path, params)?;
        let response = self
            .kernel
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("REST kernel unreachable: {error}"), None)
            })?;
        Self::json_from_response("GET", path, response).await
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<CallToolResult, McpError> {
        self.require_mutation_contract()?;
        let response = self
            .kernel
            .client()
            .post(self.kernel.url(path))
            .json(body)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("REST kernel unreachable: {error}"), None)
            })?;
        Self::tool_result_from_response("POST", path, response).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        self.require_mutation_contract()?;
        let response = self
            .kernel
            .client()
            .post(self.kernel.url(path))
            .json(body)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("REST kernel unreachable: {error}"), None)
            })?;
        Self::json_from_response("POST", path, response).await
    }

    async fn tool_result_from_response(
        method: &str,
        path: &str,
        response: reqwest::Response,
    ) -> Result<CallToolResult, McpError> {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("REST kernel {method} {path} failed with HTTP {status}: {body}"),
                None,
            ));
        }
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    async fn json_from_response(
        method: &str,
        path: &str,
        response: reqwest::Response,
    ) -> Result<serde_json::Value, McpError> {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("REST kernel {method} {path} failed with HTTP {status}: {body}"),
                None,
            ));
        }
        serde_json::from_str(&body).map_err(|error| {
            McpError::internal_error(format!("REST kernel returned invalid JSON: {error}"), None)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_session::KernelSessionClient;
    use crate::session_identity::{SessionIdentity, SessionIdentitySource};

    fn active_proxy(project_root: &str) -> CynicMcpProxy {
        CynicMcpProxy::new(
            KernelSessionClient::mock_active("/tmp/cynic-mcp-contract-test.sock", "test-agent")
                .expect("active mock session"),
            SessionIdentity {
                agent_id: "test-agent".to_string(),
                agent_type: "test".to_string(),
                source: SessionIdentitySource::GeneratedProcess,
                stable_for: "mcp_process",
                secret: false,
            },
            project_root.to_string(),
        )
    }

    #[tokio::test]
    async fn kernel_post_is_blocked_before_dispatch_when_contract_is_unobservable() {
        let error = active_proxy("/definitely-missing-cynic-project")
            .post("/coord/claim", &serde_json::json!({}))
            .await
            .expect_err("unobservable contract must block mutation before HTTP dispatch");

        assert!(
            error
                .message
                .contains("MCP runtime contract is unobservable")
        );
    }
}
