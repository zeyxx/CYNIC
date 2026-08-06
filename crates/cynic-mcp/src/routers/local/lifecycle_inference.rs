#[tool_router(router = tool_router_local_lifecycle_inference, vis = "pub(crate)")]
impl CynicMcpProxy {
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
        let constitutional_execution = agent_constitution_status();
        let unknowns = constitutional_execution
            .missing_runtime_capabilities
            .iter()
            .chain(constitutional_execution.unknowns.iter())
            .cloned()
            .collect::<Vec<_>>();

        let result = serde_json::json!({
            "status": "blocked_scheduler_unavailable",
            "mutates": false,
            "profile": profile,
            "domain": domain,
            "plan": payload_plan,
            "constitutional_execution": constitutional_execution,
            "unknowns": unknowns
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
        description = "Peer attach plane (loose coupling): probe local OpenAI-compat peers (ODS optional). Dry-run by default; persist into inference.toml SSOT only with apply=true. Schema cynic.inference.ods_discovery.v1. Does not install ODS."
    )]
    async fn cynic_inference_discover(
        &self,
        params: Parameters<InferenceDiscoverParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        let apply = params.0.apply;
        let report = cynic_inference::peer_attach(cynic_inference::PeerAttachOptions {
            apply,
            open_webui: false,
            node_id: "local-node".into(),
        })
        .await;
        let result = serde_json::to_value(&report).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_inference_vault",
        description = "List all sovereign local model profiles, GGUF file locations, assigned ports, context sizes, and whether weight files exist in the vault directory."
    )]
    async fn cynic_inference_vault(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        use cynic_inference::LocalRuntimeServiceAny;

        let service = LocalRuntimeServiceAny::from_env();
        match service.list_inventory() {
            Ok(inventory) => {
                let result = serde_json::to_value(&inventory).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                )]))
            }
            Err(error) => Err(McpError::internal_error(
                format!("failed to list model vault inventory: {error}"),
                None,
            )),
        }
    }

    #[tool(
        name = "cynic_inference_start",
        description = "Start a specific sovereign inference model profile by alias (e.g. 'qwythos-9b-q4', 'minicpm-1b-q8'). Returns convergence report with proof."
    )]
    async fn cynic_inference_start(
        &self,
        params: Parameters<InferenceStartParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        use cynic_inference::LocalRuntimeServiceAny;

        let alias = &params.0.alias;
        let wait_for_ready = params.0.wait_for_ready;
        let service = LocalRuntimeServiceAny::from_env();

        match service.start_with_transition(alias, wait_for_ready, "mcp-inference-start") {
            Ok(report) => {
                let result = serde_json::to_value(&report).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                )]))
            }
            Err(error) => Err(McpError::internal_error(
                format!("failed to start model profile '{alias}': {error}"),
                None,
            )),
        }
    }

    #[tool(
        name = "cynic_inference_stop",
        description = "Stop the currently active sovereign local inference process on the host."
    )]
    async fn cynic_inference_stop(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        use cynic_inference::LocalRuntimeServiceAny;

        let service = LocalRuntimeServiceAny::from_env();
        match service.stop(None) {
            Ok(report) => {
                let result = serde_json::to_value(&report).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                )]))
            }
            Err(error) => Err(McpError::internal_error(
                format!("failed to stop active inference process: {error}"),
                None,
            )),
        }
    }

    #[tool(
        name = "cynic_inference_switch",
        description = "Switch active inference backend to a target model alias (stops currently running backend if any, then activates target model). Enforces Human-in-the-loop confirmation for heavy models (9B+ params or >= 4GB VRAM) unless confirmed=true is passed."
    )]
    async fn cynic_inference_switch(
        &self,
        params: Parameters<InferenceSwitchParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;

        use cynic_inference::{LocalRuntimeServiceAny, SupervisionPolicy};

        let target_alias = &params.0.target_alias;
        let wait_for_ready = params.0.wait_for_ready;
        let confirmed = params.0.confirmed.unwrap_or(false);
        let service = LocalRuntimeServiceAny::from_env();

        let mut target_size = None;
        if let Ok(inventory) = service.list_inventory() {
            if let Some(profile) = inventory.models.iter().find(|p| p.alias == *target_alias) {
                target_size = profile.size_bytes;
            }
        }

        let policy = SupervisionPolicy::default();
        if policy.requires_confirmation(target_size, target_alias) && !confirmed {
            let reason = policy.confirmation_reason(target_size, target_alias).unwrap_or_default();
            let result = serde_json::json!({
                "schema": "cynic.inference.human_confirmation_required.v1",
                "status": "confirmation_required",
                "target_alias": target_alias,
                "reason": reason,
                "message": format!("Heavy model transition to '{target_alias}' requires operator confirmation. Re-run cynic_inference_switch with confirmed=true once approved."),
                "action": "pass confirmed=true to proceed with switch"
            });
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
            )]));
        }

        // Best effort stop active
        let _ = service.stop(None);

        match service.start_with_transition(target_alias, wait_for_ready, "mcp-inference-switch") {
            Ok(report) => {
                let result = serde_json::to_value(&report).unwrap_or_default();
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
                )]))
            }
            Err(error) => Err(McpError::internal_error(
                format!("failed to switch to model profile '{target_alias}': {error}"),
                None,
            )),
        }
    }
}
