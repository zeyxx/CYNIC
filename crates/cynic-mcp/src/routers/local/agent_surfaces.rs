#[tool_router(router = tool_router_local_agent_surfaces, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_mcp_surface",
        description = "Read the agent-facing CYNIC MCP surface manifest grouped by domain from infra/agent-exposure.json. Use this to discover coordination, memory/tasks, lifecycle, organism/status, and infra tools without shell inspection."
    )]
    async fn cynic_mcp_surface(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = mcp_surface_manifest(&self.project_root, &self.session_identity)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_askesis_log",
        description = "Append a typed human observation to Askesis memory for governance and reflection."
    )]
    async fn cynic_askesis_log(
        &self,
        params: Parameters<AskesisLogParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let p = params.0;
        let logfile = p
            .logfile
            .filter(|path| !path.trim().is_empty())
            .unwrap_or_else(|| {
                cynic_askesis::resolve_human_log_path()
                    .to_string_lossy()
                    .to_string()
            });

        cynic_askesis::append_human_log_entry(&logfile, &p.content, &p.domain).map_err(
            |error| {
                McpError::internal_error(format!("Failed to append Askesis log: {error}"), None)
            },
        )?;

        Ok(CallToolResult::success(vec![Content::text(
            r#"{"status": "askesis_logged"}"#,
        )]))
    }

    #[tool(
        name = "cynic_validate",
        description = "Plan the canonical `make check` gate for build_runner without compiling locally. Dispatch remains blocked until a node-owned scheduler durably enforces cynic.agent_workload_envelope.v1."
    )]
    async fn cynic_validate(
        &self,
        params: Parameters<ValidateParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let principal = self.coordination_principal()?;
        if p.agent_id
            .as_deref()
            .is_some_and(|agent_id| agent_id != principal)
        {
            return Err(McpError::invalid_params(
                "validate agent_id does not match the authenticated coordination principal",
                None,
            ));
        }
        let request_id = p.request_id.unwrap_or_else(|| {
            let short_commit = p.source_commit.chars().take(12).collect::<String>();
            format!("validate-{short_commit}-{principal}")
        });
        let request = cynic_protocol::BuildAgentRequest {
            schema: cynic_protocol::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id,
            requested_by: principal.clone(),
            source_commit: p.source_commit,
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: false,
            proof_requirements: vec![
                "canonical_gate_report".to_string(),
                "source_commit_read_back".to_string(),
                "build_runner_observation".to_string(),
            ],
            evidence_refs: p.evidence_refs,
        };
        let admission = cynic_protocol::plan_build_agent_request(&request);
        let dispatch_requested = p.dispatch;
        let constitutional_execution = agent_constitution_status();
        let result = serde_json::json!({
            "schema": "cynic.mcp.validate_dispatch.v1",
            "status": if dispatch_requested {
                "blocked_scheduler_unavailable"
            } else {
                admission.status.as_str()
            },
            "mutates": false,
            "dispatch_requested": dispatch_requested,
            "request": request,
            "admission": admission,
            "constitutional_execution": constitutional_execution
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_git",
        description = "Run safe git operations: status, diff, log, branch, stash."
    )]
    async fn cynic_git(&self, params: Parameters<GitParams>) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        // Local: runs git commands
        let result = crate::local_tools::run_git(
            self.argv_execution.as_ref(),
            &self.project_root,
            &params.0.op,
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_source_control",
        description = "Govern source material through proposal admission, worktree locking, explicit commit, and separately authorized push. Secrets and generated artifacts are rejected by the canonical protocol policy."
    )]
    async fn cynic_source_control(
        &self,
        params: Parameters<SourceControlParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let guard = crate::local_tools::source_control_lease_guard(&params.0.op)
            .map_err(|error| McpError::invalid_params(error, None))?;
        let verifier: Box<dyn cynic_host::source_lease::SourceLeaseVerifier> = match guard {
            Some(guard) => {
                let principal = self.coordination_principal()?;
                if guard.agent_id != principal {
                    Box::new(
                        cynic_host::source_lease::UnavailableSourceLeaseVerifier::new(
                            "proposal_actor_not_authenticated_principal",
                        ),
                    )
                } else {
                    let body = serde_json::json!({
                        "agent_id": guard.agent_id,
                        "target": guard.target,
                        "lease_id": guard.lease_id,
                        "fencing_token": guard.fencing_token,
                    });
                    match self
                        .post_json(cynic_protocol::coord_verify_path(), &body)
                        .await
                    {
                        Ok(value) => {
                            match serde_json::from_value::<cynic_protocol::LeaseVerification>(value)
                            {
                                Ok(verification) => Box::new(verification),
                                Err(_) => Box::new(
                                    cynic_host::source_lease::UnavailableSourceLeaseVerifier::new(
                                        "coordination_lease_verification_response_invalid",
                                    ),
                                ),
                            }
                        }
                        Err(_) => Box::new(
                            cynic_host::source_lease::UnavailableSourceLeaseVerifier::new(
                                "coordination_lease_authority_unavailable",
                            ),
                        ),
                    }
                }
            }
            None => Box::new(cynic_host::source_lease::UnavailableSourceLeaseVerifier::default()),
        };
        let result = crate::local_tools::run_source_control(
            &self.project_root,
            &params.0.op,
            verifier.as_ref(),
        );
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}
