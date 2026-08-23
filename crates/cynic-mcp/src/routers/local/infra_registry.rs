#[tool_router(router = tool_router_local_infra_registry, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "infra_registry_read",
        description = "Read CYNIC's registry desired state. Non-mutating canonical infrastructure MCP operation."
    )]
    async fn infra_registry_read(
        &self,
        params: Parameters<InfraRegistryReadParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = load_infra_registry()?;
        let selected = registry_selector(&registry, p.selector.as_deref()).ok_or_else(|| {
            McpError::invalid_params(
                format!(
                    "registry selector not found: {}",
                    p.selector.unwrap_or_default()
                ),
                None,
            )
        })?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(selected).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "infra_registry_validate",
        description = "Validate the minimal CYNIC registry and local code hygiene dimensions without mutating state. Supports network_policy, ipam, synapse, hardcoded_paths, reqwest_usage, secrets, ips, unlogged_ok, allow_without_why, and pipeline_functions."
    )]
    async fn infra_registry_validate(
        &self,
        params: Parameters<InfraRegistryValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let dimensions = p
            .dimensions
            .unwrap_or_else(|| vec!["network_policy".to_string()]);
        let registry = load_infra_registry()?;
        let needs_staged_diff = dimensions
            .iter()
            .any(|dimension| matches!(dimension.as_str(), "secrets" | "ips"));
        let staged_diff = if needs_staged_diff {
            staged_diff_content(self.argv_execution.as_ref(), &self.project_root).await
        } else {
            String::new()
        };
        let needs_staged_files = dimensions
            .iter()
            .any(|dimension| matches!(dimension.as_str(), "hardcoded_paths" | "unlogged_ok"));
        let staged_files = if needs_staged_files {
            staged_files_for_hardcoded_path_check(self.argv_execution.as_ref(), &self.project_root)
                .await
        } else {
            Vec::new()
        };
        let mut violations = Vec::new();
        for dimension in &dimensions {
            match dimension.as_str() {
                "network_policy" | "ipam" | "synapse" => {
                    violations.extend(validate_network_policy(&registry));
                }
                "hardcoded_paths" => {
                    violations.extend(validate_hardcoded_paths(
                        &self.project_root,
                        &staged_files,
                    ));
                }
                "reqwest_usage" => {
                    violations.extend(validate_reqwest_usage(&self.project_root));
                }
                "secrets" => {
                    violations.extend(validate_secrets(&staged_diff));
                }
                "ips" => {
                    violations.extend(validate_ips(&self.project_root, &staged_diff));
                }
                "unlogged_ok" => {
                    violations.extend(validate_unlogged_ok(&self.project_root, &staged_files));
                }
                "allow_without_why" => {
                    violations.extend(validate_allow_without_why(&self.project_root));
                }
                "pipeline_functions" => {
                    violations.extend(validate_pipeline_functions(&self.project_root));
                }
                unknown => violations.push(serde_json::json!({
                    "kind": "unsupported_dimension",
                    "dimension": unknown,
                    "message": "infra_registry_validate currently supports network_policy/ipam/synapse/hardcoded_paths/reqwest_usage/secrets/ips/unlogged_ok/allow_without_why/pipeline_functions"
                })),
            }
        }
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.validate.v1",
            "status": if violations.is_empty() { "ok" } else { "violation" },
            "dimensions": dimensions,
            "violations": violations,
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
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
        self.require_mutation_contract()?;

        let req = params.0;
        let response = self
            .kernel
            .client()
            .post(self.kernel.url("/lifecycle/provision"))
            .json(&req)
            .send()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response body".to_string());

        if status.is_success() {
            Ok(CallToolResult::success(vec![Content::text(body_text)]))
        } else {
            Err(McpError::internal_error(
                format!("Failed to provision infrastructure: {status} - {body_text}"),
                None,
            ))
        }
    }
}
