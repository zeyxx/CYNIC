#[tool_router(router = tool_router_local_fleet, vis = "pub(crate)")]
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
            .find(|target| target.target_id == p.target_id)
            .ok_or_else(|| McpError::invalid_params("fleet desired target not found", None))?;
        let inventory = cynic_node::fleet_inventory::build_fleet_inventory_details(
            std::path::Path::new(&registry),
        )
        .await
        .map_err(|error| McpError::internal_error(error, None))?;
        let observed =
            cynic_node::fleet_inventory::node_substrate_observation(&desired.desired, &inventory);
        let plan = cynic_node::domain::fleet_reconcile::build_plan(&desired, observed);
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
        let result =
            cynic_node::fleet_inventory::apply_proxmox_power_action(&plan.desired, action, true)
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
            .find(|target| target.target_id == p.target_id)
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
        let observed =
            cynic_node::fleet_inventory::node_substrate_observation(&desired.desired, &inventory);
        let plan = cynic_node::domain::fleet_reconcile::build_plan(&desired, observed);
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
}
