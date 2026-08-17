#[tool_router(router = tool_router_local_agent_surfaces, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_mcp_surface",
        description = "Read the agent-facing CYNIC MCP surface manifest grouped by domain from infra/agent-exposure.json. Use this to discover coordination, memory/tasks, lifecycle, organism/status, and infra tools without shell inspection."
    )]
    async fn cynic_mcp_surface(
        &self,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = mcp_surface_manifest(
            &self.project_root,
            &self.session_identity,
            !context.meta.0.is_empty(),
        )?;
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
        description = "Plan or durably deliver a canonical check, build, or release-build workload to build_runner without compiling locally. Select action=check|build|build_release; build_release also requires release_id and release_epoch and cannot complete before signed artifact-store read-back."
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
        let action = p.action.canonical_action();
        let request_prefix = p.action.request_prefix();
        let request_id = p.request_id.unwrap_or_else(|| {
            let short_commit = p.source_commit.chars().take(12).collect::<String>();
            format!("{request_prefix}-{short_commit}-{principal}")
        });
        let dispatch_requested = p.dispatch;
        let release_publication = if action == "make:build-release" {
            let release_id = p.release_id.ok_or_else(|| {
                McpError::invalid_params("release_id is required for action=build_release", None)
            })?;
            let release_epoch = p.release_epoch.ok_or_else(|| {
                McpError::invalid_params("release_epoch is required for action=build_release", None)
            })?;
            Some(cynic_protocol::BuildReleasePublicationIntent {
                schema: cynic_protocol::BUILD_RELEASE_PUBLICATION_INTENT_SCHEMA.to_string(),
                release_id,
                release_epoch,
            })
        } else {
            if p.release_id.is_some() || p.release_epoch.is_some() {
                return Err(McpError::invalid_params(
                    "release_id and release_epoch are only valid for action=build_release",
                    None,
                ));
            }
            None
        };
        let mut proof_requirements = if action == "make:build-release" {
            vec![
                "release_binary_artifact_read_back".to_string(),
                "signed_release_basket_read_back".to_string(),
                "source_commit_read_back".to_string(),
                "build_runner_observation".to_string(),
            ]
        } else {
            vec![
                "canonical_gate_report".to_string(),
                "source_commit_read_back".to_string(),
                "build_runner_observation".to_string(),
            ]
        };
        let source = cynic_host::repo_access::load_forgejo_desired_state(std::path::Path::new(
            &self.project_root,
        ))
        .map_err(|error| {
            McpError::internal_error(
                format!("build source authority is unavailable: {error}"),
                None,
            )
        })?;
        let source_commit = p.source_commit;
        let snapshot = match (
            p.snapshot_digest,
            p.snapshot_authority_ref,
            p.snapshot_locator,
        ) {
            (Some(digest), Some(authority_ref), Some(locator)) => {
                proof_requirements.push("source_snapshot_digest_read_back".to_string());
                Some(cynic_protocol::BuildSourceSnapshotReference {
                    schema: cynic_protocol::BUILD_SOURCE_SNAPSHOT_REFERENCE_SCHEMA.to_string(),
                    digest,
                    format: cynic_protocol::SOURCE_MATERIALIZATION_FORMAT.to_string(),
                    authority_ref,
                    source: Some(cynic_protocol::RuntimeArtifactSource {
                        transport: cynic_protocol::RuntimeArtifactTransportKind::Https,
                        locator,
                    }),
                })
            }
            (None, None, None) => None,
            _ => {
                return Err(McpError::invalid_params(
                    "snapshot_digest, snapshot_authority_ref, and snapshot_locator must be supplied together",
                    None,
                ));
            }
        };
        let request = cynic_protocol::BuildAgentRequest {
            schema: cynic_protocol::BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id,
            requested_by: principal.clone(),
            source_commit: source_commit.clone(),
            source: cynic_protocol::BuildSourceReference {
                schema: cynic_protocol::BUILD_SOURCE_REFERENCE_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                transport_uri: source.git_ssh_url,
                revision: source_commit,
                authority_ref: "infra/repo-remotes.json:forgejo".to_string(),
                snapshot,
            },
            action: action.to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized: dispatch_requested,
            proof_requirements,
            release_publication,
            evidence_refs: p.evidence_refs,
        };
        let admission = if dispatch_requested {
            cynic_protocol::admit_build_agent_request(&request)
        } else {
            cynic_protocol::plan_build_agent_request(&request)
        };
        let constitutional_execution = agent_constitution_status();
        if !dispatch_requested || admission.status == "blocked" {
            let result = serde_json::json!({
                "schema": "cynic.mcp.validate_dispatch.v2",
                "status": admission.status,
                "mutates": false,
                "dispatch_requested": dispatch_requested,
                "request": request,
                "admission": admission,
                "constitutional_execution": constitutional_execution
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
            )]));
        }
        let registry = std::path::Path::new(&self.project_root).join("infra/registry.json");
        let registry = registry.to_string_lossy().to_string();
        let transport = cynic_host::registry::resolve_build_runner_transport(Some(&registry))
            .map_err(|error| McpError::internal_error(error, None))?;
        let envelope = cynic_protocol::compose_build_agent_workload(
            &request,
            &transport.node_id,
            "/var/lib/cynic/source/workspaces",
        )
        .map_err(|blockers| {
            McpError::invalid_params(
                format!("build workload composition blocked: {}", blockers.join(",")),
                None,
            )
        })?;
        let delivery = cynic_protocol::AgentWorkloadDelivery {
            schema: cynic_protocol::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            target_node_id: transport.node_id,
            submitted_by: principal,
            submitted_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
            status: cynic_protocol::AgentWorkloadDeliveryStatus::Pending,
            accepted_at_ms: None,
            accepted_record_digest: None,
            acceptance_lease: None,
            terminal_at_ms: None,
            terminal_record_digest: None,
            terminal_record: None,
            completion: None,
            completion_evidence: Vec::new(),
            evidence_refs: request.evidence_refs.clone(),
            envelope,
        };
        let response = self
            .kernel
            .client()
            .post(self.kernel.url("/agent-workloads"))
            .json(&delivery)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("workload delivery failed: {error}"), None)
            })?;
        let response_status = response.status();
        let body = response.text().await.map_err(|error| {
            McpError::internal_error(format!("workload delivery read-back failed: {error}"), None)
        })?;
        let delivery_read_back = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| serde_json::json!({"raw": body}));
        let result = serde_json::json!({
            "schema": "cynic.mcp.validate_dispatch.v2",
            "status": if response_status.is_success() { "delivered" } else { "blocked_delivery_unavailable" },
            "mutates": response_status.is_success(),
            "dispatch_requested": dispatch_requested,
            "request": request,
            "admission": admission,
            "delivery": delivery,
            "delivery_read_back": delivery_read_back,
            "kernel_http_status": response_status.as_u16(),
            "constitutional_execution": constitutional_execution,
            "execution_status": "waiting_for_target_node_acceptance"
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_agent_workload_complete",
        description = "Publish a terminal AgentWorkloadCompletionPublication to the kernel for a previously accepted workload delivery. Requires node_id, workload_id, and the serialized completion publication payload. Persists the completion receipt via the node-control endpoint and returns the kernel read-back proof."
    )]
    async fn cynic_agent_workload_complete(
        &self,
        params: Parameters<AgentWorkloadCompleteParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        if p.node_id.trim().is_empty() {
            return Err(McpError::invalid_params(
                "node_id is required and must not be empty",
                None,
            ));
        }
        if p.workload_id.trim().is_empty() {
            return Err(McpError::invalid_params(
                "workload_id is required and must not be empty",
                None,
            ));
        }
        let publication: cynic_protocol::AgentWorkloadCompletionPublication =
            serde_json::from_value(p.publication).map_err(|error| {
                McpError::invalid_params(
                    format!(
                        "publication is not a valid AgentWorkloadCompletionPublication: {error}"
                    ),
                    None,
                )
            })?;
        if publication.workload_id != p.workload_id || publication.node_id != p.node_id {
            return Err(McpError::invalid_params(
                "publication workload_id/node_id must match the path parameters",
                None,
            ));
        }
        let path = format!(
            "/node-control/{}/agent-workloads/{}/completion",
            p.node_id, p.workload_id
        );
        let response = self
            .kernel
            .client()
            .post(self.kernel.url(&path))
            .json(&publication)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(
                    format!("workload completion publication failed: {error}"),
                    None,
                )
            })?;
        let response_status = response.status();
        let body = response.text().await.map_err(|error| {
            McpError::internal_error(
                format!("workload completion read-back failed: {error}"),
                None,
            )
        })?;
        let read_back = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| serde_json::json!({"raw": body}));
        let result = serde_json::json!({
            "schema": "cynic.mcp.agent_workload_complete.v1",
            "workload_id": p.workload_id,
            "node_id": p.node_id,
            "kernel_http_status": response_status.as_u16(),
            "status": if response_status.is_success() {
                "terminal_read_back"
            } else {
                "completion_blocked"
            },
            "mutates": response_status.is_success(),
            "completion_read_back": read_back
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_git",
        description = "Run read-only git observations: status, diff, and log. Source mutation uses cynic_source_control with an admitted lease."
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
