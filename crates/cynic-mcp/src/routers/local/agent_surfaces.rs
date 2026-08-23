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
        name = "cynic_fleet_discover",
        description = "Discover declared CYNIC fleet nodes and probe their services via TCP. Reads infra/registry.json (declared state), probes each node's services, and returns {node_id, role, ip, status, last_seen} per node. Read-only. Source of truth = registry (declared), not nmap."
    )]
    async fn cynic_fleet_discover(
        &self,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let registry_path = self.project_root.clone() + "/infra/registry.json";
        let adapter = cynic_host::fleet_discovery::FleetDiscoveryAdapter::new(registry_path);
        let report = adapter.discover();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_local_autonomy",
        description = "Observe local autonomy state: OS health, kernel reachability, mesh peers. Returns the current AutonomyMode (Coordinated/Autonomous/Degraded) and retained capabilities. Kernel-optional: works even if the kernel is dead."
    )]
    async fn cynic_local_autonomy(
        &self,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let registry_path = self.project_root.clone() + "/infra/registry.json";
        let runtime = cynic_host::local_autonomy_runtime::LocalAutonomyRuntime::new(
            "local".to_string(),
            registry_path,
        );
        let decision = runtime.tick();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&decision).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_agent_runtime_observe",
        description = "Observe declared external agent runtimes (hermes-agent, mercury-agent, etc.) with payload provenance, supervisor gaps, and adoption plan. Read-only; never starts a daemon during observation. Kernel-optional: works even if the kernel REST endpoint is unreachable."
    )]
    async fn cynic_agent_runtime_observe(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let report = cynic_host::agent_runtime::observe_agent_runtimes(
            &cynic_host::update_lifecycle::LocalArgvRunner,
        );
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_emergence_elect",
        description = "Elect roles via emergence (deterministic, same on every node). When the kernel is dead, nodes self-coordinate: each node floods its local state, and every node runs the same election rule. Returns {assumed_role, role_priority, candidates, reasoning} per role. No consensus protocol, no leader election — just deterministic emergence from local observations."
    )]
    async fn cynic_emergence_elect(
        &self,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let registry_path = self.project_root.clone() + "/infra/registry.json";
        let mut runtime = cynic_host::emergence_runtime::EmergenceRuntime::new(
            "local".to_string(),
            registry_path,
        );
        let decisions = runtime.tick();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&decisions).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_build_run",
        description = "Trigger a distributed build via the sovereign CI/CD pipeline. Observes the build runner, verifies the lease, runs the pure workflow decision (admit source → admit runner → decide), and returns the build proof. Requires --apply to execute; without it, returns the planned proof only. Read-back proof is signed and content-addressed."
    )]
    async fn cynic_build_run(
        &self,
        params: Parameters<crate::types::BuildTriggerParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let result = crate::local_tools::run_buildWorkflow(
            self.argv_execution.as_ref(),
            &self.project_root,
            &p.commit,
            p.phase.as_deref(),
            p.network_mode.as_deref(),
            p.host.as_deref(),
            p.user.as_deref(),
        )
        .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_judgment",
        description = "Project the runtime's current state into a single CynicJudgment through three orthogonal lenses: DIKW (Data/Information/Knowledge/Wisdom), SAGE (Priming/Sense/Simulate/Act/Evaluate), and Cynefin (Clear/Complicated/Complex/Chaotic/Disordered). Read-only. Returns the judgment as machine-readable JSON."
    )]
    async fn cynic_judgment(
        &self,
        params: Parameters<crate::types::JudgmentParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let p = params.0;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let subject = p.subject.as_deref().unwrap_or("local");
        let mut j = cynic_protocol::CynicJudgment::new(
            format!("mcp-judgment-{subject}-{now}"),
            false,
            cynic_protocol::judgment::DijkwLayer::Information,
            cynic_protocol::judgment::SagePhase::Sense,
            cynic_protocol::judgment::CynefinDomain::Complicated,
            now,
        );
        j.residual_unknowns = vec![
            "os_health_observation_pending_host_integration".to_string(),
            "convergence_matrix_observation_pending".to_string(),
        ];
        j.confidence = 0.3;
        let layers = p.promote_layers.unwrap_or(0);
        for _ in 0..layers {
            if let Some(promoted) = j.promote_layer() {
                j = promoted;
            } else {
                break;
            }
        }
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&j).unwrap_or_default(),
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
        self.require_mutation_contract()?;

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
        if dispatch_requested {
            self.require_mutation_contract()?;
        }
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
        let (status, execution_status) = if response_status.is_success() {
            ("delivered", "waiting_for_target_node_acceptance")
        } else if response_status == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            (
                "blocked_kernel_method_not_allowed",
                "blocked_kernel_workload_delivery_method_unavailable",
            )
        } else {
            (
                "blocked_delivery_unavailable",
                "blocked_kernel_workload_delivery_unavailable",
            )
        };
        let result = serde_json::json!({
            "schema": "cynic.mcp.validate_dispatch.v2",
            "status": status,
            "mutates": response_status.is_success(),
            "dispatch_requested": dispatch_requested,
            "request": request,
            "admission": admission,
            "delivery": delivery,
            "delivery_read_back": delivery_read_back,
            "kernel_http_status": response_status.as_u16(),
            "constitutional_execution": constitutional_execution,
            "execution_status": execution_status
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

    #[tool(
        name = "cynic_source_change_submit",
        description = "Validate and deliver one content-addressed source-change workload to its node-owned scheduler. MCP never applies the change or writes a worktree."
    )]
    async fn cynic_source_change_submit(
        &self,
        params: Parameters<crate::types::SourceChangeSubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        self.require_mutation_contract()?;
        let principal = self.coordination_principal()?;
        let p = params.0;
        let workload = serde_json::from_value::<cynic_protocol::AgentWorkloadEnvelope>(p.workload)
            .map_err(|error| {
                McpError::invalid_params(format!("invalid source-change workload: {error}"), None)
            })?;
        let submission =
            serde_json::from_value::<cynic_protocol::SourceChangeProposal>(p.submission)
                .map_err(|error| {
                    McpError::invalid_params(
                        format!("invalid source-change submission: {error}"),
                        None,
                    )
                })?;
        if workload.delegation.agent_id != principal {
            return Err(McpError::invalid_params(
                "source-change workload agent_id does not match the authenticated coordination principal",
                None,
            ));
        }
        let admission = cynic_protocol::admit_source_change(&submission);
        if admission.status != "admitted" {
            let result = serde_json::json!({
                "schema": "cynic.mcp.source_change_submission.v1",
                "status": "blocked",
                "mutates": false,
                "admission": admission,
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
            )]));
        }
        let delivery = cynic_protocol::AgentWorkloadDelivery {
            schema: cynic_protocol::AGENT_WORKLOAD_DELIVERY_SCHEMA.to_string(),
            target_node_id: workload.delegation.node_id.clone(),
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
            evidence_refs: submission.evidence_refs.clone(),
            envelope: workload,
        };
        let response = self
            .kernel
            .client()
            .post(self.kernel.url("/agent-workloads"))
            .json(&delivery)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(
                    format!("source-change workload delivery failed: {error}"),
                    None,
                )
            })?;
        let response_status = response.status();
        let body = response.text().await.map_err(|error| {
            McpError::internal_error(
                format!("source-change workload read-back failed: {error}"),
                None,
            )
        })?;
        let delivery_read_back = serde_json::from_str::<serde_json::Value>(&body)
            .unwrap_or_else(|_| serde_json::json!({"raw": body}));
        let result = serde_json::json!({
            "schema": "cynic.mcp.source_change_submission.v1",
            "status": if response_status.is_success() { "delivered" } else { "blocked_delivery_unavailable" },
            "mutates": response_status.is_success(),
            "admission": admission,
            "delivery": delivery,
            "delivery_read_back": delivery_read_back,
            "kernel_http_status": response_status.as_u16(),
            "execution_status": if response_status.is_success() {
                "waiting_for_target_node_acceptance"
            } else {
                "blocked_kernel_workload_delivery_unavailable"
            },
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_federation_status",
        description = "Observe the current federation reconciliation: operating mode (coordinated/autonomous/degraded/promoted), retained capabilities, assumed roles, takeover status, residual unknowns. This is the single authoritative composition of node_control, local_autonomy, emergence, and runtime_takeover — not four competing models."
    )]
    async fn cynic_federation_status(
        &self,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = crate::local_tools::run_federation_status().await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_node_activation",
        description = "Activate a node — bounded action with read-back proof. Plans the activation (bootstrap/advance/takeover), checks consent (--yes), executes argv, performs read-back, returns proof (Activated/Failed/RolledBack). Mutates host — requires explicit consent."
    )]
    async fn cynic_node_activation(
        &self,
        params: Parameters<NodeActivationParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let Parameters(NodeActivationParams { node_id, yes }) = params;
        if yes {
            self.require_mutation_contract()?;
        }
        let result = crate::local_tools::run_node_activation(&node_id, yes).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_node_activation_status",
        description = "Observe node activation state: compose deployment_epoch + bootstrap_federation + runtime_takeover into a single verdict (Active/Activatable/Blocked). Identifies blockers preventing activation."
    )]
    async fn cynic_node_activation_status(
        &self,
        params: Parameters<NodeActivationStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let Parameters(NodeActivationStatusParams { node_id }) = params;
        let result = crate::local_tools::run_node_activation_status(&node_id).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_fleet_health",
        description = "Aggregate fleet health from heartbeats. Detects silent nodes (no heartbeat within timeout), unexpected nodes (heartbeat but not in registry), and network partitions. Returns alive/degraded/unreachable counts, silent list, partition detection flag."
    )]
    async fn cynic_fleet_health(
        &self,
        Parameters(params): Parameters<crate::types::FleetHealthParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = crate::local_tools::run_fleet_health(params.heartbeat_timeout_s).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_inference_status",
        description = "Report inference configuration status: routing SSOT path, backends (name, sovereign, base_url, model, context_size, has_api_key), sovereign count. Read-only projection of the inference SSOT."
    )]
    async fn cynic_inference_status(
        &self,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = crate::local_tools::run_inference_status().await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_storage_credential_sync",
        description = "Evaluate whether SurrealDB storage credentials are synchronized across service config, password file, and live auth. Diagnoses the known bug where /etc/cynic/secrets/storage-runtime-password contained a hash instead of plaintext 'cynic' — root cause of digestion_starvation. Returns SYNCHRONIZED, OUT_OF_SYNC (with repair action), or UNKNOWN."
    )]
    async fn cynic_storage_credential_sync(
        &self,
        Parameters(params): Parameters<crate::types::StorageCredentialSyncParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let endpoint = params.endpoint;
        let result = crate::local_tools::run_storage_credential_sync(&endpoint).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_harness_progression",
        description = "Evaluate whether the harness can progress from one kind to the next (lab → conformance → staging → canary → production). Requires explicit proof categories — no silent promotion. Returns ALLOWED with authority level or BLOCKED with reason."
    )]
    async fn cynic_harness_progression(
        &self,
        Parameters(params): Parameters<crate::types::HarnessProgressionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = crate::local_tools::run_harness_progression(
            &params.current_kind,
            &params.target_kind,
            &params.satisfied,
            &params.unknown,
        ).await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}
