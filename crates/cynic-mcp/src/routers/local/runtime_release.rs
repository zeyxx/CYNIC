#[tool_router(router = tool_router_local_runtime_release, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_runtime_release_declare",
        description = "Plan or apply runtime desired state from a retained terminal release-build proof. CYNIC derives policy_digest, re-admits the exact completion, and only then projects NodeDesiredState."
    )]
    async fn cynic_runtime_release_declare(
        &self,
        params: Parameters<RuntimeReleaseDeclareParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let principal = self.coordination_principal()?;
        let mut desired = serde_json::from_value::<cynic_protocol::NodeDesiredState>(
            p.desired_state,
        )
        .map_err(|error| {
            McpError::invalid_params(format!("invalid node desired state: {error}"), None)
        })?;
        desired.policy_digest = cynic_protocol::runtime_release_policy_digest(&desired)
            .map_err(|error| McpError::invalid_params(error, None))?;
        let declaration = cynic_protocol::RuntimeReleaseDeclaration {
            schema: cynic_protocol::RUNTIME_RELEASE_DECLARATION_SCHEMA.to_string(),
            requested_by: principal,
            workload_id: p.workload_id,
            desired_state: desired,
        };
        let response = self
            .kernel
            .client()
            .get(
                self.kernel
                    .url(&format!("/agent-workloads/{}", declaration.workload_id)),
            )
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("read terminal workload: {error}"), None)
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            McpError::internal_error(format!("read terminal workload response: {error}"), None)
        })?;
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("terminal workload endpoint returned {status}: {body}"),
                None,
            ));
        }
        let delivery = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| value.get("value").cloned())
            .and_then(|value| {
                serde_json::from_value::<cynic_protocol::AgentWorkloadDelivery>(value).ok()
            })
            .ok_or_else(|| {
                McpError::internal_error("terminal workload read-back is invalid", None)
            })?;
        let admission = cynic_protocol::admit_runtime_release(&declaration, &delivery);
        if !p.apply || !admission.admitted {
            let result = serde_json::json!({
                "schema": "cynic.mcp.runtime_release_declaration.v1",
                "status": if admission.admitted { "planned" } else { "blocked" },
                "mutates": false,
                "apply_requested": p.apply,
                "declaration": declaration,
                "admission": admission,
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
            )]));
        }
        let target_node_id = declaration.desired_state.node_id.clone();
        let response = self
            .kernel
            .client()
            .put(
                self.kernel
                    .url(&format!("/node-control/{target_node_id}/desired-state")),
            )
            .json(&declaration)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("declare runtime release: {error}"), None)
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            McpError::internal_error(format!("read runtime release response: {error}"), None)
        })?;
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("runtime release endpoint returned {status}: {body}"),
                None,
            ));
        }
        let result = serde_json::from_str::<serde_json::Value>(&body).map_err(|error| {
            McpError::internal_error(format!("decode runtime release response: {error}"), None)
        })?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }
}
