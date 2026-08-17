#[tool_router(router = tool_router_local_services, vis = "pub(crate)")]
impl CynicMcpProxy {
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
        let provider_payload = self.provider_cluster_resources().await;
        let observed = observed_resource_for_node(&registry, node, &provider_payload);
        let observed_status = observed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str());
        let endpoint = service_endpoint_from_registry(&registry, node, service);
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
        let result = service_health_report(
            &registry,
            node,
            &p.service,
            service,
            p.endpoint,
            p.timeout_seconds,
        )
        .await?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "storage_primary_plan",
        description = "Render the non-mutating dedicated storage primary activation plan from the Rust-native storage controller. Does not install or start storage."
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
        description = "Build a local dedicated storage activation bundle under /tmp. Writes only the local artifact; does not install or start storage."
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
                let provider_payload = self.provider_cluster_resources().await;
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
                        &registry,
                        node,
                        "surrealdb",
                        service,
                        None,
                        status_params.service_health_timeout_seconds,
                    )
                    .await?,
                );
                let artifact_store = find_declared_service(node, "runtime-artifact-store")
                    .ok_or_else(|| {
                        McpError::internal_error(
                            "runtime artifact store service missing from storage primary node",
                            None,
                        )
                    })?;
                evidence.insert(
                    "artifact_store".to_string(),
                    service_health_report(
                        &registry,
                        node,
                        "runtime-artifact-store",
                        artifact_store,
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
}
