#[tool_router(router = tool_router_local_storage, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "storage_primary_status",
        description = "Read the Rust-native storage primary activation status. Non-mutating canonical storage.primary.status operation."
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
                let local_projection = run_lxc_exec_bridge_status(&LxcExecBridgeStatusParams {
                    helper_path: None,
                    installed_policy_path: None,
                    sudoers_path: None,
                })?;
                let bridge_status = Self::observe_lxc_exec_bridge_transport(
                    self.argv_execution.as_ref(),
                    local_projection,
                )
                .await;
                evidence.insert("bridge_status".to_string(), bridge_status);
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
                        params.service_health_timeout_seconds,
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
}
