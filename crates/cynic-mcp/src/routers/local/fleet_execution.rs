#[tool_router(router = tool_router_local_fleet_execution, vis = "pub(crate)")]
impl CynicMcpProxy {
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
        let provider_payload = self.provider_cluster_resources().await;
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
        description = "Render the non-mutating installation plan for the Rust-native bounded LXC bridge. Does not install helper or sudoers."
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
        description = "Build a local LXC exec bridge activation bundle under /tmp. Writes only the local artifact; does not install helper, policy, or sudoers."
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
        description = "Read the Rust-native LXC bridge helper/sudoers contract and MCP transport configuration without mutation."
    )]
    async fn lxc_exec_bridge_status(
        &self,
        params: Parameters<LxcExecBridgeStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let local_projection = run_lxc_exec_bridge_status(&params.0)?;
        let result =
            Self::observe_lxc_exec_bridge_transport(self.argv_execution.as_ref(), local_projection)
                .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "fleet_node_exec",
        description = "Execute exact admitted argv through the canonical fleet.node.exec preflight. QEMU uses the guest agent; LXC uses the Rust-native bounded bridge with pinned SSH trust and explicit apply for mutations."
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
                        "argv": &p.argv,
                        "agent_id": p.agent_id.as_deref().unwrap_or("mcp-fleet-node-exec"),
                        "timeout_seconds": p.timeout_seconds.unwrap_or(30).clamp(1, 120),
                        "transport_kind": transport_kind,
                    });
                    let exec = Self::run_host_exec_transport(
                        self.argv_execution.as_ref(),
                        &argv,
                        &request,
                    )
                    .await?;
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
        let provider_payload = self.provider_cluster_resources().await;
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
            let Some((ssh_target, helper, known_hosts)) = Self::lxc_exec_bridge_config() else {
                let result = serde_json::json!({
                    "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                    "status": "precondition_missing",
                    "node": node_name,
                    "proxmox_id": proxmox_id,
                    "purpose": &purpose,
                    "missing": "lxc_exec_bridge_transport",
                    "required_bridge": "install the Rust-native bounded helper, then set CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET and CYNIC_LXC_EXEC_BRIDGE_KNOWN_HOSTS",
                    "reason": "Raw SSH/sudo/pct execution is not a canonical portable MCP operation.",
                    "observed": observed,
                });
                return Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_default(),
                )]));
            };
            let argv = validate_lxc_argv(&p.argv)
                .map_err(|message| McpError::invalid_params(message, None))?;
            let vmid_number = vmid.parse::<u64>().map_err(|_| {
                McpError::invalid_params(
                    format!("invalid LXC vmid in resource id: {proxmox_id}"),
                    None,
                )
            })?;
            let request = serde_json::json!({
                "schema": "cynic.lxc_exec_bridge.request.v1",
                "vmid": vmid_number,
                "purpose": &purpose,
                "argv": argv,
                "agent_id": p.agent_id.as_deref().unwrap_or("mcp-fleet-node-exec"),
                "timeout_seconds": p.timeout_seconds.unwrap_or(30).clamp(1, 60),
                "apply": p.apply.unwrap_or(false),
                "authority": p.authority.as_deref().unwrap_or("runtime_operator"),
            });
            let exec = Self::run_lxc_exec_bridge(
                self.argv_execution.as_ref(),
                &ssh_target,
                &helper,
                &known_hosts,
                &request,
            )
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
                    "proxmox_api_node": proxmox_api_node,
                    "lxc_exec_bridge_transport": "rust_native_bounded_ssh_helper"
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

        if !p.apply.unwrap_or(false) {
            let result = serde_json::json!({
                "schema": "cynic.infrastructure_mcp.fleet_node_exec.v1",
                "status": "mutation_authorization_required",
                "node": node_name,
                "proxmox_id": proxmox_id,
                "purpose": &purpose,
                "reason": "QEMU guest-agent exec cannot prove an argv is read-only; explicit apply=true is required.",
                "observed": observed,
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )]));
        }
        let timeout_seconds = p.timeout_seconds.unwrap_or(30).clamp(1, 120);
        let exec = self
            .provider_qemu_guest_exec(proxmox_api_node, vmid, &p.argv, timeout_seconds)
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
}
