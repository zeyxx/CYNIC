#[tool_router(router = tool_router_local_operator_transport, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_operator_transport_status",
        description = "Read local SSH bootstrap and tmux operator-harness evidence. Read-only: SSH remains fallback-only and tmux never grants runtime mutation authority."
    )]
    async fn cynic_operator_transport_status(
        &self,
        _params: Parameters<OperatorTransportStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let observer =
            cynic_host::adapters::local_operator_transport::LocalOperatorTransportObserver;
        let result = cynic_host::transport::build_transport_matrix_report(&observer);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_transport_security_audit",
        description = "Audit organism transport membranes and fail closed for remote mutation when confidentiality, peer identity, mutual authentication, or authorization binding is not proven. Read-only; unknowns remain explicit."
    )]
    async fn cynic_transport_security_audit(
        &self,
        _params: Parameters<OperatorTransportStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = cynic_host::transport::build_transport_security_report();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_forgejo_status",
        description = "Observe the declared sovereign Forgejo endpoint, desired Git-over-SSH remote, reachability, and authentication readiness without reading or returning credentials. Read-only; observed service health is not treated as source-promotion authority."
    )]
    async fn cynic_forgejo_status(
        &self,
        _params: Parameters<OperatorTransportStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = cynic_host::repo_access::current_repo_access_status();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_forgejo_plan",
        description = "Build the read-only convergence plan for the sovereign Forgejo service from typed authority and observed runtime evidence. Unknown API, repository, key, or SSH authentication state remains blocking; no token is read and no mutation is performed."
    )]
    async fn cynic_forgejo_plan(
        &self,
        _params: Parameters<OperatorTransportStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = cynic_host::forgejo_control::current_forgejo_plan()
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_ssh_target_inspect",
        description = "Read public SSH host-key fingerprints for a bootstrap target without accepting or persisting trust. Agents must return the fingerprint for independent operator comparison before any connection attempt."
    )]
    async fn cynic_ssh_target_inspect(
        &self,
        params: Parameters<SshTargetInspectParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let observer =
            cynic_host::adapters::local_operator_transport::LocalOperatorTransportObserver;
        let result = cynic_host::transport::inspect_ssh_target(&observer, &params.0.host);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}
