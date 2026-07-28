// ── Local tools (no REST forwarding) ────────────────────────

#[tool_router(router = tool_router_local, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_fleet_reconcile_apply",
        description = "Apply one declared fleet power reconciliation only after rebuilding its plan, binding admission to its digest, and receiving explicit apply=true. Produces provider task/read-back evidence."
    )]
    async fn cynic_fleet_reconcile_apply(
        &self,
        params: Parameters<FleetReconcileApplyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = p.registry.unwrap_or_else(|| {
            std::path::Path::new(&self.project_root)
                .join("infra/registry.json")
                .display()
                .to_string()
        });
        let targets = cynic_node::domain::fleet_reconcile::load_desired_targets(
            std::path::Path::new(&registry),
        )
        .map_err(|error| McpError::invalid_params(error, None))?;
        let desired = targets
            .into_iter()
            .find(|target| target.id == p.target_id)
            .ok_or_else(|| McpError::invalid_params("fleet desired target not found", None))?;
        let inventory = cynic_node::fleet_inventory::build_fleet_inventory_details(
            std::path::Path::new(&registry),
        )
        .await
        .map_err(|error| McpError::internal_error(error, None))?;
        let observed = inventory
            .nodes
            .into_iter()
            .map(|node| cynic_node::domain::fleet_truth::ObservedResource {
                provider: node.provider,
                name: node.name,
                status: node.status,
                resource_id: Some(node.node_id),
                kind: None,
                constellation: None,
            })
            .collect::<Vec<_>>();
        let plan = cynic_node::domain::fleet_reconcile::build_plan(&desired, &observed);
        let admission = cynic_node::domain::fleet_reconcile::admit_plan(&plan, p.actor.clone());
        let authorization = cynic_protocol::ExecutionAuthorization::new(
            p.actor,
            cynic_protocol::PolicyAuthority::RuntimeAuthority,
            p.apply,
            Vec::new(),
        );
        let gate =
            cynic_node::domain::fleet_reconcile::authorize_apply(&plan, &admission, &authorization);
        if !gate.admitted() {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(
                    &serde_json::json!({"plan": plan, "admission": admission, "gate": gate}),
                )
                .unwrap_or_default(),
            )]));
        }
        let action = plan
            .actions
            .first()
            .ok_or_else(|| McpError::internal_error("admitted plan has no action", None))?;
        let result = cynic_node::fleet_inventory::apply_proxmox_power_action(action, true)
            .await
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(CallToolResult::success(vec![Content::text(serde_json::to_string_pretty(&serde_json::json!({"plan": plan, "admission": admission, "gate": gate, "proof": result})).unwrap_or_default())]))
    }

    #[tool(
        name = "cynic_fleet_reconcile_plan",
        description = "Build a non-mutating provider-agnostic fleet reconciliation plan from registry desired state and live provider observation. It never starts, stops, creates, or destroys a guest."
    )]
    async fn cynic_fleet_reconcile_plan(
        &self,
        params: Parameters<FleetReconcilePlanParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = p.registry.unwrap_or_else(|| {
            std::path::Path::new(&self.project_root)
                .join("infra/registry.json")
                .display()
                .to_string()
        });
        let targets = cynic_node::domain::fleet_reconcile::load_desired_targets(
            std::path::Path::new(&registry),
        )
        .map_err(|error| McpError::invalid_params(error, None))?;
        let desired = targets
            .into_iter()
            .find(|target| target.id == p.target_id)
            .ok_or_else(|| {
                McpError::invalid_params(
                    format!("fleet desired target not found: {}", p.target_id),
                    None,
                )
            })?;
        let inventory = cynic_node::fleet_inventory::build_fleet_inventory_details(
            std::path::Path::new(&registry),
        )
        .await
        .map_err(|error| McpError::internal_error(error, None))?;
        let observed = inventory
            .nodes
            .into_iter()
            .map(|node| cynic_node::domain::fleet_truth::ObservedResource {
                provider: node.provider,
                name: node.name,
                status: node.status,
                resource_id: Some(node.node_id),
                kind: node
                    .meta
                    .as_ref()
                    .and_then(|meta| meta.get("vm_type"))
                    .cloned(),
                constellation: None,
            })
            .collect::<Vec<_>>();
        let plan = cynic_node::domain::fleet_reconcile::build_plan(&desired, &observed);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&plan).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_fleet_inventory",
        description = "Observe declared fleet state against configured provider adapters. Read-only: provider credentials are injected, never returned; standard TLS or an injected CA bundle is required."
    )]
    async fn cynic_fleet_inventory(
        &self,
        params: Parameters<FleetInventoryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let registry = params.0.registry.unwrap_or_else(|| {
            std::path::Path::new(&self.project_root)
                .join("infra/registry.json")
                .display()
                .to_string()
        });
        let result = cynic_node::fleet_inventory::build_fleet_inventory_details(
            std::path::Path::new(&registry),
        )
        .await
        .map_err(|error| McpError::internal_error(error, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

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

    #[tool(
        name = "cynic_mcp_surface",
        description = "Read the agent-facing CYNIC MCP surface manifest grouped by domain from infra/agent-exposure.json. Use this to discover coordination, memory/tasks, lifecycle, organism/status, and infra tools without shell inspection."
    )]
    async fn cynic_mcp_surface(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = mcp_surface_manifest(&self.project_root)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_auth",
        description = "Fallback manual authentication for MCP sessions missing CYNIC_API_KEY env forwarding. Prefer fixing the MCP wrapper instead of passing secrets through the model."
    )]
    async fn cynic_auth(&self, params: Parameters<AuthParams>) -> Result<CallToolResult, McpError> {
        let p = params.0;
        // Auth is local: compare against env var, don't forward to REST
        let expected = std::env::var("CYNIC_API_KEY").unwrap_or_default();
        if expected.is_empty() {
            return Err(McpError::internal_error(
                "Kernel has no CYNIC_API_KEY configured",
                None,
            ));
        }
        if p.api_key != expected {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Authentication failed — invalid API key",
                None,
            ));
        }
        self.authenticated.store(true, Ordering::Relaxed);
        Ok(CallToolResult::success(vec![Content::text(
            r#"{"authenticated": true}"#,
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
            "constitutional_execution": {
                "schema": cynic_protocol::AGENT_WORKLOAD_ENVELOPE_SCHEMA,
                "scheduler_status": "inactive",
                "legacy_agent_task_dispatch_used": false,
                "missing_capabilities": [
                    "durable_agent_workload_store",
                    "aggregate_sibling_budget_accounting",
                    "lease_fenced_claim_and_recovery",
                    "durable_cancellation",
                    "completion_evidence_persistence"
                ]
            }
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
        let result = crate::local_tools::run_git(&self.project_root, &params.0.op).await;
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
        let mut violations = Vec::new();
        for dimension in &dimensions {
            match dimension.as_str() {
                "network_policy" | "ipam" | "synapse" => {
                    violations.extend(validate_network_policy(&registry));
                }
                "hardcoded_paths" => {
                    violations.extend(validate_hardcoded_paths(&self.project_root));
                }
                "reqwest_usage" => {
                    violations.extend(validate_reqwest_usage(&self.project_root));
                }
                "secrets" => {
                    violations.extend(validate_secrets(&self.project_root));
                }
                "ips" => {
                    violations.extend(validate_ips(&self.project_root));
                }
                "unlogged_ok" => {
                    violations.extend(validate_unlogged_ok(&self.project_root));
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
        description = "Provisions infrastructure targets (like storage_primary or kernel_primary) autonomously. Use this when you detect an infrastructure outage. If MCP is unavailable, fallback to the local 'cynic infra provision <target>' command."
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
            .post(format!("{}/lifecycle/provision", self.base_url))
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

    #[tool(
        name = "network_ipam_check",
        description = "Check a registry node's canonical service-plane reservation. Non-mutating network.ipam.check operation."
    )]
    async fn network_ipam_check(
        &self,
        params: Parameters<NetworkIpamCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let plane = p.plane.unwrap_or_else(|| "synapse".to_string());
        if plane != "synapse" {
            return Err(McpError::invalid_params(
                "network_ipam_check v1 only supports plane=synapse",
                None,
            ));
        }
        let registry = load_infra_registry()?;
        let node = find_registry_node(&registry, &p.node).ok_or_else(|| {
            McpError::invalid_params(format!("registry node not found: {}", p.node), None)
        })?;
        let node_name = node_name(node).unwrap_or(&p.node);
        let synapse = node
            .get("network")
            .and_then(|network| network.get("synapse"))
            .and_then(|value| value.as_str());
        let reservation = find_reservation(&registry, node_name);
        let cidr = synapse_cidr(&registry);
        let mut policy_violations = Vec::new();
        if synapse.is_none() {
            policy_violations.push(serde_json::json!({"kind": "missing_node_synapse"}));
        }
        if reservation.is_none() {
            policy_violations.push(serde_json::json!({"kind": "missing_reservation"}));
        }
        if let (Some(synapse), Some(cidr)) = (synapse, cidr)
            && !synapse.starts_with('<')
            && !ipv4_in_cidr(synapse, cidr)
        {
            policy_violations.push(serde_json::json!({
                "kind": "synapse_ip_outside_cidr",
                "synapse": synapse,
                "cidr": cidr
            }));
        }
        if let (Some(synapse), Some(reservation)) = (synapse, reservation)
            && reservation.get("synapse").and_then(|value| value.as_str()) != Some(synapse)
        {
            policy_violations.push(serde_json::json!({
                "kind": "reservation_mismatch",
                "node_synapse": synapse,
                "reservation_synapse": reservation.get("synapse")
            }));
        }
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.network_ipam_check.v1",
            "node": node_name,
            "plane": plane,
            "reserved": reservation.is_some() && policy_violations.is_empty(),
            "synapse": synapse,
            "cidr": cidr,
            "dns": synapse_dns_names(node, reservation),
            "policy_violations": policy_violations,
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_dns_render",
        description = "Render the Synapse DNS records from registry policy without writing files. Non-mutating network.mesh.dns.render operation."
    )]
    async fn network_mesh_dns_render(
        &self,
        params: Parameters<NetworkMeshDnsRenderParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        if let Some(registry_ref) = &p.registry_ref
            && registry_ref != "infra/registry.json"
        {
            return Err(McpError::invalid_params(
                "network_mesh_dns_render v1 only supports registry_ref=infra/registry.json",
                None,
            ));
        }
        let registry = load_infra_registry()?;
        let violations = validate_network_policy(&registry);
        let records = render_synapse_dns_records(&registry, p.include_short_names.unwrap_or(true));
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.network_mesh_dns_render.v1",
            "zone": "synapse.cynic",
            "registry_ref": "infra/registry.json",
            "policy_status": if violations.is_empty() { "ok" } else { "violation" },
            "violations": violations,
            "records": records,
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_status",
        description = "Read deterministic Synapse/WireGuard mesh status from registry policy and optional DNS resolution. Non-mutating network.mesh.status operation."
    )]
    async fn network_mesh_status(
        &self,
        params: Parameters<NetworkMeshStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        if let Some(registry_ref) = &p.registry_ref
            && registry_ref != "infra/registry.json"
        {
            return Err(McpError::invalid_params(
                "network_mesh_status v1 only supports registry_ref=infra/registry.json",
                None,
            ));
        }
        let registry = load_infra_registry()?;
        let result = network_mesh_status_report(
            &registry,
            p.resolve_dns.unwrap_or(false),
            p.include_records.unwrap_or(true),
        );
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_dns_activation_plan",
        description = "Render the non-mutating Synapse DNS activation plan from the canonical cynic-node network surface. Does not install CoreDNS or resolver configuration."
    )]
    async fn network_mesh_dns_activation_plan(
        &self,
        params: Parameters<NetworkMeshDnsActivationPlanParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_dns_activation_plan(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_dns_activation_bundle",
        description = "Build a local non-mutating Synapse DNS activation bundle under /tmp from the canonical cynic-node network surface. Does not install CoreDNS or resolver configuration."
    )]
    async fn network_mesh_dns_activation_bundle(
        &self,
        params: Parameters<NetworkMeshDnsActivationBundleParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_dns_activation_bundle(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_provider_fallback_hosts",
        description = "Build a local non-mutating provider-plane hosts fallback bundle under /tmp from the canonical cynic-node network surface. Temporary only while Synapse DNS is inactive."
    )]
    async fn network_mesh_provider_fallback_hosts(
        &self,
        params: Parameters<NetworkMeshProviderFallbackHostsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_provider_fallback_hosts(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_provider_fallback_status",
        description = "Read provider-plane fallback resurrection status: /etc/hosts entries, persistent route, and optional service reachability."
    )]
    async fn network_mesh_provider_fallback_status(
        &self,
        params: Parameters<NetworkMeshProviderFallbackStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_provider_fallback_status(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_dns_activation_status",
        description = "Read non-mutating Synapse DNS activation status from the canonical cynic-node network surface. Reports bundle/service/resolver checks and optional DNS resolution."
    )]
    async fn network_mesh_dns_activation_status(
        &self,
        params: Parameters<NetworkMeshDnsActivationStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_dns_activation_status(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "fleet_node_status",
        description = "Read a registry node's declared/effective status from registry plus provider evidence. Non-mutating canonical fleet.node.status operation."
    )]
    async fn fleet_node_status(
        &self,
        params: Parameters<FleetNodeStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = load_infra_registry()?;
        let node = find_registry_node(&registry, &p.node).ok_or_else(|| {
            McpError::invalid_params(format!("registry node not found: {}", p.node), None)
        })?;
        let declared_status = node
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let proxmox_id = proxmox_resource_id_for_node(&registry, node);
        let provider_payload = self.proxmox_cluster_resources().await;
        let observed = observed_resource_for_node(&registry, node, &provider_payload);
        let observed_status = observed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str());
        let effective_status = effective_status_from_provider(declared_status, observed_status);
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.fleet_node_status.v1",
            "node": node_name(node),
            "proxmox_id": proxmox_id,
            "declared": node,
            "observed": observed,
            "effective_status": effective_status,
            "provider_evidence": provider_payload,
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "lxc_exec_bridge_plan",
        description = "Render the non-mutating LXC exec bridge installation plan from organ-fleet. Does not install helper, policy, or sudoers."
    )]
    async fn lxc_exec_bridge_plan(
        &self,
        params: Parameters<LxcExecBridgePlanParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_lxc_exec_bridge_plan(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "lxc_exec_bridge_bundle",
        description = "Build a local non-mutating LXC exec bridge activation bundle under /tmp. Does not install helper, policy, or sudoers."
    )]
    async fn lxc_exec_bridge_bundle(
        &self,
        params: Parameters<LxcExecBridgeBundleParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_lxc_exec_bridge_bundle(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "lxc_exec_bridge_status",
        description = "Read non-mutating LXC exec bridge activation status from the organ-fleet contract. Reports missing helper/policy/sudoers and MCP transport configuration."
    )]
    async fn lxc_exec_bridge_status(
        &self,
        params: Parameters<LxcExecBridgeStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_lxc_exec_bridge_status(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "fleet_node_exec",
        description = "Execute a command through the canonical fleet.node.exec preflight. QEMU uses Proxmox guest-agent; LXC returns a structured precondition error until an LXC exec bridge is declared."
    )]
    async fn fleet_node_exec(
        &self,
        params: Parameters<FleetNodeExecParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let purpose = p.purpose.clone();
        let registry = load_infra_registry()?;
        let node = find_registry_node(&registry, &p.node).ok_or_else(|| {
            McpError::invalid_params(format!("registry node not found: {}", p.node), None)
        })?;
        let node_name = node_name(node).unwrap_or(&p.node);
        let Some(proxmox_id) = proxmox_resource_id_for_node(&registry, node) else {
            if node_requires_host_exec_transport(node) {
                let declared_transport = host_exec_transport_for_node(node).cloned();
                let transport_kind = host_exec_transport_kind(node).unwrap_or("unknown");
                let required_bridge = declared_transport
                    .as_ref()
                    .and_then(|value| value.get("required_bridge"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("declare a canonical host exec transport for cortex/manual hosts before using fleet_node_exec on this node");
                if matches!(transport_kind, "service_agent_surface" | "ssh_sudo_helper") {
                    let argv = match host_exec_transport_command_argv(node) {
                        Ok(Some(argv)) => argv,
                        Ok(None) => {
                            let result = serde_json::json!({
                                "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                                "status": "precondition_missing",
                                "node": node_name,
                                "purpose": &purpose,
                                "missing": "host_exec_transport_command",
                                "declared_role": node.get("role").and_then(|value| value.as_str()),
                                "declared_transport_kind": transport_kind,
                                "reason": "Registry transport kind is executable but no helper command is configured.",
                                "required_bridge": required_bridge,
                                "declared_transport": declared_transport,
                                "declared": node,
                            });
                            return Ok(CallToolResult::success(vec![Content::text(
                                serde_json::to_string_pretty(&result).unwrap_or_default(),
                            )]));
                        }
                        Err(message) => {
                            return Err(McpError::invalid_params(message, None));
                        }
                    };
                    let request = serde_json::json!({
                        "schema": "cynic.host_exec_transport.request.v1",
                        "node": node_name,
                        "role": node.get("role").and_then(|value| value.as_str()),
                        "purpose": &purpose,
                        "command": &p.command,
                        "agent_id": p.agent_id.as_deref().unwrap_or("mcp-fleet-node-exec"),
                        "timeout_seconds": p.timeout_seconds.unwrap_or(30).clamp(1, 120),
                        "transport_kind": transport_kind,
                    });
                    let exec = Self::run_host_exec_transport(&argv, &request)?;
                    let result = serde_json::json!({
                        "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                        "status": exec.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
                        "node": node_name,
                        "purpose": &purpose,
                        "preconditions": {
                            "host_exec_transport_declared": true,
                            "host_exec_transport_kind": transport_kind,
                            "host_exec_transport_command": argv,
                        },
                        "exec": exec,
                        "declared_transport": declared_transport,
                        "audit_event": {
                            "operation": "fleet.node.exec",
                            "node": node_name,
                            "purpose": &purpose,
                            "provider": "host_exec_transport"
                        }
                    });
                    return Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap_or_default(),
                    )]));
                }
                let result = serde_json::json!({
                    "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                    "status": "precondition_missing",
                    "node": node_name,
                    "purpose": &purpose,
                    "missing": "host_exec_transport",
                    "declared_transport_kind": transport_kind,
                    "declared_role": node.get("role").and_then(|value| value.as_str()),
                    "reason": "Registry node is a host-native surface, not a Proxmox guest resource. fleet_node_exec currently covers Proxmox guests and the LXC exec bridge only.",
                    "required_bridge": required_bridge,
                    "declared_transport": declared_transport,
                    "declared": node,
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_default(),
                )]));
            }
            return Err(McpError::invalid_params(
                format!("registry node has no Proxmox resource id: {node_name}"),
                None,
            ));
        };
        let provider_payload = self.proxmox_cluster_resources().await;
        let observed = observed_resource_for_node(&registry, node, &provider_payload);
        let observed_status = observed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str());

        if observed_status != Some("running") {
            let result = serde_json::json!({
                "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                "status": "precondition_failed",
                "node": node_name,
                "proxmox_id": proxmox_id,
                "purpose": &purpose,
                "precondition": "node_observed_running",
                "observed": observed,
                "provider_evidence": provider_payload,
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )]));
        }

        let Some((kind, vmid)) = proxmox_resource_kind_id(&proxmox_id) else {
            return Err(McpError::invalid_params(
                format!("unsupported Proxmox resource id: {proxmox_id}"),
                None,
            ));
        };
        let proxmox_api_node = observed
            .as_ref()
            .and_then(|value| value.get("node"))
            .and_then(|value| value.as_str())
            .or_else(|| node.get("host").and_then(|value| value.as_str()))
            .unwrap_or("pve");
        if kind == "lxc" {
            let Some((ssh_target, helper)) = Self::lxc_exec_bridge_config() else {
                let result = serde_json::json!({
                    "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                    "status": "precondition_missing",
                    "node": node_name,
                    "proxmox_id": proxmox_id,
                    "purpose": &purpose,
                    "missing": "lxc_exec_bridge_transport",
                    "required_bridge": "set CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET after installing the root-owned helper and sudoers contract on the Proxmox host",
                    "reason": "Raw SSH/sudo/pct execution is not a canonical portable MCP operation.",
                    "observed": observed,
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_default(),
                )]));
            };
            let argv = lxc_command_argv(&p.command)
                .map_err(|message| McpError::invalid_params(message, None))?;
            let vmid_number = vmid.parse::<u64>().map_err(|_| {
                McpError::invalid_params(
                    format!("invalid LXC vmid in resource id: {proxmox_id}"),
                    None,
                )
            })?;
            let request = serde_json::json!({
                "node": proxmox_api_node,
                "vmid": vmid_number,
                "purpose": &purpose,
                "argv": argv,
                "agent_id": p.agent_id.as_deref().unwrap_or("mcp-fleet-node-exec"),
                "timeout_seconds": p.timeout_seconds.unwrap_or(30).clamp(1, 60),
            });
            let exec = Self::run_lxc_exec_bridge(&ssh_target, &helper, &request)?;
            let result = serde_json::json!({
                "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                "status": exec.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
                "node": node_name,
                "proxmox_id": proxmox_id,
                "purpose": &purpose,
                "preconditions": {
                    "node_observed_running": true,
                    "resource_kind": kind,
                    "proxmox_api_node": proxmox_api_node,
                    "lxc_exec_bridge_transport": "ssh_sudo_helper"
                },
                "exec": exec,
                "observed": observed,
                "audit_event": {
                    "operation": "fleet.node.exec",
                    "node": node_name,
                    "purpose": &purpose,
                    "provider": "lxc_exec_bridge"
                }
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )]));
        }

        let timeout_seconds = p.timeout_seconds.unwrap_or(30).clamp(1, 120);
        let exec = self
            .proxmox_qemu_agent_exec(proxmox_api_node, vmid, &p.command, timeout_seconds)
            .await?;
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
            "status": exec.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
            "node": node_name,
            "proxmox_id": proxmox_id,
            "purpose": &purpose,
            "preconditions": {
                "node_observed_running": true,
                "resource_kind": kind,
                "proxmox_api_node": proxmox_api_node
            },
            "exec": exec,
            "audit_event": {
                "operation": "fleet.node.exec",
                "node": node_name,
                "purpose": &purpose,
                "provider": "proxmox_qemu_guest_agent"
            }
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "service_status",
        description = "Read declared service status from registry plus node provider status. Non-mutating canonical service.status operation."
    )]
    async fn service_status(
        &self,
        params: Parameters<ServiceStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = load_infra_registry()?;
        let node = find_registry_node(&registry, &p.node).ok_or_else(|| {
            McpError::invalid_params(format!("registry node not found: {}", p.node), None)
        })?;
        let service = find_declared_service(node, &p.service).ok_or_else(|| {
            McpError::invalid_params(
                format!(
                    "service '{}' is not declared on node '{}'",
                    p.service, p.node
                ),
                None,
            )
        })?;
        let provider_payload = self.proxmox_cluster_resources().await;
        let observed = observed_resource_for_node(&registry, node, &provider_payload);
        let observed_status = observed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str());
        let endpoint = service_endpoint_from_registry(node, service);
        let result = serde_json::json!({
            "schema": "cynic.infrastructure_mcp.service_status.v1",
            "node": node_name(node),
            "service": p.service,
            "declared": service,
            "endpoint": endpoint,
            "node_effective_status": effective_status_from_provider(
                node.get("status").and_then(|value| value.as_str()).unwrap_or("unknown"),
                observed_status
            ),
            "observed_node": observed,
            "systemd_state": serde_json::Value::Null,
            "evidence": {
                "registry": "infra/registry.json",
                "provider": provider_payload,
                "systemd": "not_observed_without_fleet_node_exec_or_service_agent"
            }
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "service_health",
        description = "Check a declared service health endpoint without falling back to provider/LAN addresses. Non-mutating canonical service.health operation."
    )]
    async fn service_health(
        &self,
        params: Parameters<ServiceHealthParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let registry = load_infra_registry()?;
        let node = find_registry_node(&registry, &p.node).ok_or_else(|| {
            McpError::invalid_params(format!("registry node not found: {}", p.node), None)
        })?;
        let service = find_declared_service(node, &p.service).ok_or_else(|| {
            McpError::invalid_params(
                format!(
                    "service '{}' is not declared on node '{}'",
                    p.service, p.node
                ),
                None,
            )
        })?;
        let result =
            service_health_report(node, &p.service, service, p.endpoint, p.timeout_seconds).await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "storage_primary_plan",
        description = "Render the non-mutating dedicated storage primary activation plan from organ-keep. Does not install or start storage."
    )]
    async fn storage_primary_plan(
        &self,
        params: Parameters<StoragePrimaryPlanParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_storage_primary_plan(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "storage_primary_bundle",
        description = "Build a local non-mutating dedicated storage activation bundle under /tmp. Does not install or start storage."
    )]
    async fn storage_primary_bundle(
        &self,
        params: Parameters<StoragePrimaryBundleParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_storage_primary_bundle(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "storage_primary_readiness",
        description = "Read aggregated dedicated storage readiness by composing bundle, bridge, fleet, mesh and service health evidence. Non-mutating."
    )]
    async fn storage_primary_readiness(
        &self,
        params: Parameters<StoragePrimaryReadinessParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let status_params = storage_primary_readiness_status_params(&params.0);
        let use_auto_bridge = storage_primary_status_uses_auto_bridge(&status_params);
        let use_auto_mesh = storage_primary_status_uses_auto_mesh(&status_params);
        let use_auto_fleet = storage_primary_status_uses_auto_fleet(&status_params);
        let use_auto_health = storage_primary_status_uses_auto_health(&status_params);
        let evidence = if use_auto_bridge || use_auto_mesh || use_auto_fleet || use_auto_health {
            let registry = load_infra_registry()?;
            let mut evidence = serde_json::Map::new();
            if use_auto_bridge {
                evidence.insert(
                    "bridge_status".to_string(),
                    run_lxc_exec_bridge_status(&LxcExecBridgeStatusParams {
                        helper_path: None,
                        installed_policy_path: None,
                        sudoers_path: None,
                    })?,
                );
            }
            if use_auto_mesh {
                evidence.insert(
                    "mesh_status".to_string(),
                    network_mesh_status_report(
                        &registry,
                        status_params.resolve_mesh_dns.unwrap_or(true),
                        false,
                    ),
                );
            }
            if use_auto_fleet {
                let provider_payload = self.proxmox_cluster_resources().await;
                evidence.insert(
                    "fleet_report".to_string(),
                    storage_fleet_report(&registry, &provider_payload)?,
                );
            }
            if use_auto_health {
                let node =
                    find_registry_node(&registry, "cynic-lxc-surrealdb").ok_or_else(|| {
                        McpError::internal_error("storage primary node missing from registry", None)
                    })?;
                let service = find_declared_service(node, "surrealdb").ok_or_else(|| {
                    McpError::internal_error(
                        "surrealdb service missing from storage primary node",
                        None,
                    )
                })?;
                evidence.insert(
                    "health".to_string(),
                    service_health_report(
                        node,
                        "surrealdb",
                        service,
                        None,
                        status_params.service_health_timeout_seconds,
                    )
                    .await?,
                );
            }
            Some(serde_json::Value::Object(evidence))
        } else {
            None
        };
        let status = run_storage_primary_status(&status_params, evidence)?;
        let mut result = storage_primary_readiness_result(status);
        if use_auto_mesh {
            let dns_activation_status =
                run_network_mesh_dns_activation_status(&NetworkMeshDnsActivationStatusParams {
                    hosts_path: None,
                    corefile_path: None,
                    service_path: None,
                    resolved_dropin_path: None,
                    resolve_dns: Some(status_params.resolve_mesh_dns.unwrap_or(true)),
                })?;
            if let Some(object) = result.as_object_mut() {
                let evidence = object
                    .entry("evidence")
                    .or_insert_with(|| serde_json::json!({}));
                if let Some(evidence_object) = evidence.as_object_mut() {
                    evidence_object
                        .insert("dns_activation_status".to_string(), dns_activation_status);
                }
            }
        }
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "storage_primary_status",
        description = "Read organ-keep storage primary activation status. Non-mutating canonical storage.primary.status operation."
    )]
    async fn storage_primary_status(
        &self,
        params: Parameters<StoragePrimaryStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let params = params.0;
        let use_auto_bridge = storage_primary_status_uses_auto_bridge(&params);
        let use_auto_mesh = storage_primary_status_uses_auto_mesh(&params);
        let use_auto_fleet = storage_primary_status_uses_auto_fleet(&params);
        let use_auto_health = storage_primary_status_uses_auto_health(&params);
        let evidence = if use_auto_bridge || use_auto_mesh || use_auto_fleet || use_auto_health {
            let registry = load_infra_registry()?;
            let mut evidence = serde_json::Map::new();
            if use_auto_bridge {
                evidence.insert(
                    "bridge_status".to_string(),
                    run_lxc_exec_bridge_status(&LxcExecBridgeStatusParams {
                        helper_path: None,
                        installed_policy_path: None,
                        sudoers_path: None,
                    })?,
                );
            }
            if use_auto_mesh {
                evidence.insert(
                    "mesh_status".to_string(),
                    network_mesh_status_report(
                        &registry,
                        params.resolve_mesh_dns.unwrap_or(false),
                        false,
                    ),
                );
            }
            if use_auto_fleet {
                let provider_payload = self.proxmox_cluster_resources().await;
                evidence.insert(
                    "fleet_report".to_string(),
                    storage_fleet_report(&registry, &provider_payload)?,
                );
            }
            if use_auto_health {
                let node =
                    find_registry_node(&registry, "cynic-lxc-surrealdb").ok_or_else(|| {
                        McpError::internal_error("storage primary node missing from registry", None)
                    })?;
                let service = find_declared_service(node, "surrealdb").ok_or_else(|| {
                    McpError::internal_error(
                        "surrealdb service missing from storage primary node",
                        None,
                    )
                })?;
                evidence.insert(
                    "health".to_string(),
                    service_health_report(
                        node,
                        "surrealdb",
                        service,
                        None,
                        params.service_health_timeout_seconds,
                    )
                    .await?,
                );
            }
            Some(serde_json::Value::Object(evidence))
        } else {
            None
        };
        let result = run_storage_primary_status(&params, evidence)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_list",
            description = "List all VMs and LXC containers on the Proxmox hypervisor. Returns their status, memory, and ID."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_list(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_list")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        // 1. Get nodes
        let nodes_resp = client
            .get(format!("{proxmox_url}/api2/json/nodes"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Proxmox API failed (nodes): {e}"), None)
            })?;

        let nodes_json: serde_json::Value = nodes_resp.json().await.map_err(|e| {
            McpError::internal_error(format!("Invalid JSON from Proxmox: {e}"), None)
        })?;

        let mut output = String::new();

        if let Some(nodes) = nodes_json["data"].as_array() {
            for node in nodes {
                if let Some(node_name) = node["node"].as_str() {
                    output.push_str(&format!("=== Node: {node_name} ===\n"));

                    // 2. Get VMs
                    let qemu_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/qemu"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = qemu_resp
                        && let Ok(qemu_json) = resp.json::<serde_json::Value>().await
                    {
                        output.push_str("VMs:\n");
                        if let Some(vms) = qemu_json["data"].as_array() {
                            for vm in vms {
                                let vmid = vm["vmid"].as_u64().unwrap_or(0);
                                let name = vm["name"].as_str().unwrap_or("unknown");
                                let status = vm["status"].as_str().unwrap_or("unknown");
                                let mem = vm["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                output.push_str(&format!(
                                    "- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"
                                ));
                            }
                        }
                    }

                    // 3. Get LXCs
                    let lxc_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/lxc"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = lxc_resp
                        && let Ok(lxc_json) = resp.json::<serde_json::Value>().await
                    {
                        output.push_str("Containers (LXC):\n");
                        if let Some(lxcs) = lxc_json["data"].as_array() {
                            for lxc in lxcs {
                                let vmid = lxc["vmid"].as_u64().unwrap_or(0);
                                let name = lxc["name"].as_str().unwrap_or("unknown");
                                let status = lxc["status"].as_str().unwrap_or("unknown");
                                let mem = lxc["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                output.push_str(&format!(
                                    "- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_action",
            description = "Perform actions (start, stop, reboot, suspend, resume) on Proxmox VMs and LXC containers."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_action(
        &self,
        params: Parameters<ProxmoxActionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_action")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;
        let vmid = p.vmid;
        let node = p.node;
        let type_ = p.target_type; // "qemu" or "lxc"
        let action = p.action; // "start", "stop", "reboot", "suspend", "resume"

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let url = format!("{proxmox_url}/api2/json/nodes/{node}/{type_}/{vmid}/status/{action}");

        let resp = client
            .post(&url)
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_provision",
            description = "Provision a new Proxmox VM from an archive backup (vma.gz). Provides autonomous infrastructure cloning."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_provision(
        &self,
        params: Parameters<ProxmoxProvisionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_provision")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(300)) // Provisioning takes time
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let url = format!("{proxmox_url}/api2/json/nodes/{}/qemu", p.node);

        // pvesh create /nodes/pve/qemu -vmid 130 -archive local:backup/... -name cynic-vm-staging -storage local-lvm
        let form = [
            ("vmid", p.vmid.to_string()),
            ("archive", p.archive),
            ("name", p.name),
            ("storage", p.storage),
            ("force", "1".to_string()), // Force overwriting existing if any
        ];

        let resp = client
            .post(&url)
            .header("Authorization", &auth_header)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_exec",
            description = "Execute a command inside a Proxmox VM using the QEMU Guest Agent without requiring SSH."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_exec(
        &self,
        params: Parameters<ProxmoxExecParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_exec")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let exec_url = format!(
            "{proxmox_url}/api2/json/nodes/{}/qemu/{}/agent/exec",
            p.node, p.vmid
        );

        // Split command into arguments (simple split by space, for complex commands use sh -c)
        let mut form = Vec::new();
        for arg in p.command.split_whitespace() {
            form.push(("command", arg.to_string()));
        }

        let resp = client
            .post(&exec_url)
            .header("Authorization", &auth_header)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
        let pid = json["data"]["pid"].as_i64().ok_or_else(|| {
            McpError::internal_error(
                format!("Failed to get PID from exec response: {text}"),
                None,
            )
        })?;

        let status_url = format!(
            "{proxmox_url}/api2/json/nodes/{}/qemu/{}/agent/exec-status?pid={}",
            p.node, p.vmid, pid
        );

        // Wait for execution to finish (up to ~30 seconds)
        for _ in 0..15 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let status_resp = client
                .get(&status_url)
                .header("Authorization", &auth_header)
                .send()
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("Proxmox API GET failed: {e}"), None)
                })?;

            let status_text = status_resp.text().await.unwrap_or_default();
            let status_json: serde_json::Value =
                serde_json::from_str(&status_text).unwrap_or(serde_json::json!({}));

            if status_json["data"]["exited"].as_i64() == Some(1) {
                let exitcode = status_json["data"]["exitcode"].as_i64().unwrap_or(-1);
                let out_data = status_json["data"]["out-data"].as_str().unwrap_or("");
                let err_data = status_json["data"]["err-data"].as_str().unwrap_or("");

                let result_text =
                    format!("Exit code: {exitcode}\nStdout:\n{out_data}\nStderr:\n{err_data}");
                return Ok(CallToolResult::success(vec![Content::text(result_text)]));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Command timed out after 30 seconds. PID was {pid}."
        ))]))
    }

    #[tool(
        name = "cynic_lifecycle_status",
        description = "Read a concise registry lifecycle status for humans and agents: ready state, dispatch permission, targets, debt summary, and next commands."
    )]
    async fn cynic_lifecycle_status(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_lifecycle_status()?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_lifecycle_dispatch",
        description = "Plan a canonical lifecycle workload. Execution dispatch is blocked until the node-owned constitutional scheduler is active."
    )]
    async fn cynic_lifecycle_dispatch(
        &self,
        params: Parameters<LifecycleDispatchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let profile = params.0.profile;

        let payload_plan = run_lifecycle_payload_plan(&profile)?;
        let payloads = lifecycle_payloads_from_plan(&payload_plan)?;
        let domain = lifecycle_payload_domain(payloads, &profile);

        let result = serde_json::json!({
            "status": "blocked_scheduler_unavailable",
            "mutates": false,
            "profile": profile,
            "domain": domain,
            "plan": payload_plan,
            "constitutional_execution": {
                "schema": cynic_protocol::AGENT_WORKLOAD_ENVELOPE_SCHEMA,
                "scheduler_status": "inactive",
                "legacy_agent_task_dispatch_used": false
            },
            "unknowns": [
                "durable_agent_workload_store",
                "aggregate_sibling_budget_accounting",
                "lease_fenced_claim_and_recovery",
                "durable_cancellation",
                "completion_evidence_persistence"
            ]
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_lifecycle_payloads",
        description = "Dry-run a canonical lifecycle profile and return registry-derived task payloads without writing to the Kernel mempool."
    )]
    async fn cynic_lifecycle_payloads(
        &self,
        params: Parameters<LifecycleDispatchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let profile = params.0.profile;
        let payload_plan = run_lifecycle_payload_plan(&profile)?;
        lifecycle_payloads_from_plan(&payload_plan)?;
        let result = lifecycle_payload_dry_run_result(payload_plan);
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_inference_backends",
        description = "Read the sovereign inference configuration from the SSOT (CYNIC_INFERENCE_CONFIG or /etc/cynic/inference.toml) and probe each backend health endpoint. Returns declared backends with sovereign flag, model, base_url, and live health status. Never emits API keys. Use this to discover which local inference backends are available before routing tasks."
    )]
    async fn cynic_inference_backends(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        use cynic_inference::{InferenceConfigError, load_inference_config};

        let loaded = match load_inference_config() {
            Ok(loaded) => loaded,
            Err(InferenceConfigError::NotFound { path }) => {
                let result = serde_json::json!({
                    "schema": "cynic.inference_backends.v1",
                    "status": "not_configured",
                    "config_path": path.to_string_lossy(),
                    "message": "No inference config found. Copy infra/inference.toml.example to the reported path.",
                    "backends": [],
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                )]));
            }
            Err(error) => {
                return Err(McpError::internal_error(
                    format!("inference config error: {error}"),
                    None,
                ));
            }
        };

        // Probe each backend /models — 3s timeout, best-effort, non-blocking.
        #[allow(clippy::expect_used)] // WHY: default() only fails on TLS init, unrecoverable
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap_or_default();

        let mut backend_views = Vec::new();
        for backend in &loaded.backends {
            let health_url = backend
                .health_url
                .clone()
                .unwrap_or_else(|| format!("{}/models", backend.base_url.trim_end_matches('/')));

            let health_status = match http.get(&health_url).send().await {
                Ok(resp) if resp.status().is_success() => "healthy",
                Ok(_) => "degraded",
                Err(_) => "unreachable",
            };

            backend_views.push(serde_json::json!({
                "name": backend.name,
                "sovereign": backend.sovereign,
                "base_url": backend.base_url,
                "model": backend.model,
                "backend_type": format!("{:?}", backend.backend_type).to_lowercase(),
                "context_size": backend.context_size,
                "timeout_secs": backend.timeout_secs,
                "max_tokens": backend.max_tokens,
                "has_api_key": backend.api_key.is_some(),
                "health": health_status,
            }));
        }

        let sovereign_healthy = backend_views
            .iter()
            .filter(|b| {
                b.get("sovereign")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                    && b.get("health").and_then(|v| v.as_str()) == Some("healthy")
            })
            .count();

        let result = serde_json::json!({
            "schema": "cynic.inference_backends.v1",
            "status": "ok",
            "config_path": loaded.path.map(|p| p.to_string_lossy().into_owned()),
            "backend_count": backend_views.len(),
            "sovereign_healthy_count": sovereign_healthy,
            "backends": backend_views,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_inference_discover",
        description = "Peer attach plane (loose coupling): probe local OpenAI-compat peers (ODS optional). Default applies into inference.toml SSOT. Schema cynic.inference.ods_discovery.v1. Does not install ODS."
    )]
    async fn cynic_inference_discover(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        // MCP tools are mutation-tier: apply into SSOT (toml only; no Open WebUI side effect).
        let report = cynic_inference::peer_attach(cynic_inference::PeerAttachOptions {
            apply: true,
            open_webui: false,
            node_id: "local-node".into(),
        })
        .await;
        let result = serde_json::to_value(&report).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }
}
