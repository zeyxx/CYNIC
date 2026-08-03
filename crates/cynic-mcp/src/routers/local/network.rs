#[tool_router(router = tool_router_local_network, vis = "pub(crate)")]
impl CynicMcpProxy {
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
        description = "Build a local Synapse DNS activation bundle under /tmp. Writes only the local artifact; does not mutate the host network, install CoreDNS, or change resolver configuration."
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
        name = "network_mesh_dns_activation_apply",
        description = "Plan or apply the exact digest-bound Synapse DNS bundle on the current node. Mutation requires apply=true and root bootstrap authority; every file and service action is read back and rolled back on failure."
    )]
    async fn network_mesh_dns_activation_apply(
        &self,
        params: Parameters<NetworkMeshDnsActivationApplyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result = run_network_mesh_dns_activation_apply(&params.0)?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "network_mesh_provider_fallback_hosts",
        description = "Build a local provider-plane hosts fallback bundle under /tmp. Writes only the local artifact; does not mutate hosts/routes. Temporary only while Synapse DNS is inactive."
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
        name = "provider_control_status",
        description = "Observe whether the injected Proxmox provider-control authority is enrolled and usable. Read-only; reports credential presence and API read-back without returning secrets or secret paths."
    )]
    async fn provider_control_status(
        &self,
        _params: Parameters<ProviderControlStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let result =
            cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::observe_from_env(
                std::time::Duration::from_secs(10),
            )
            .await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }
}
