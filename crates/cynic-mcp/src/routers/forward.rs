// ── Forwarded tools (REST kernel) ───────────────────────────

#[tool_router(router = tool_router_forward, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_api_contracts",
        description = "Discover CYNIC's machine-readable API contracts through the kernel: manifest, OpenAPI, API topology, or layer alignment. No auth required."
    )]
    async fn cynic_api_contracts(
        &self,
        params: Parameters<ApiContractsParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let contract = params.0.contract.unwrap_or_else(|| "manifest".to_string());
        match contract.as_str() {
            "manifest" => Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "schema": "cynic.api_contracts_manifest.v1",
                    "contracts": {
                        "openapi": "/openapi.json",
                        "topology": "/api/topology",
                        "layer_alignment": "/api/layer-alignment"
                    },
                    "valid_contract_values": ["manifest", "openapi", "topology", "layer_alignment"]
                })
                .to_string(),
            )])),
            "openapi" => self.get("/openapi.json").await,
            "topology" => self.get("/api/topology").await,
            "layer_alignment" => self.get("/api/layer-alignment").await,
            other => Err(McpError::invalid_params(
                format!(
                    "unknown contract {other:?}; expected manifest, openapi, topology, or layer_alignment"
                ),
                None,
            )),
        }
    }

    #[tool(
        name = "cynic_judge",
        description = "Submit content for epistemic evaluation by CYNIC's independent AI validators (Dogs). Returns φ-bounded Q-Score, per-axiom breakdown (FIDELITY/PHI/VERIFY/CULTURE/BURN/SOVEREIGNTY), verdict (HOWL/WAG/GROWL/BARK), and multi-model reasoning. Max confidence: 61.8%."
    )]
    async fn cynic_judge(
        &self,
        params: Parameters<JudgeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        self.post(
            "/judge",
            &serde_json::json!({
                "content": p.content,
                "context": p.context,
                "domain": p.domain,
                "dogs": p.dogs,
                "crystals": p.crystals,
                "priority": p.priority,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_health",
        description = "Get CYNIC system health: Dog status, quality rates, crystal counts, version. No auth required."
    )]
    async fn cynic_health(&self) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        self.get("/health").await
    }

    #[tool(
        name = "cynic_attention",
        description = "Cortex startup attention surface: compactly summarizes fleet liveness, degraded dogs, overdue tasks, metabolism pressure, and next actions from kernel health + mempool. Use at session start instead of manually inspecting full /health JSON."
    )]
    async fn cynic_attention(
        &self,
        params: Parameters<AttentionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        let task_kind = p.task_kind.unwrap_or_else(|| "cortex".to_string());
        let task_limit = p.task_limit.unwrap_or(10).clamp(1, 25).to_string();
        let mut task_query = vec![("task_kind", task_kind), ("task_limit", task_limit)];
        if let Some(domain) = p.task_domain {
            task_query.push(("task_domain", domain));
        }
        self.get_query("/attention", &task_query).await
    }

    #[tool(
        name = "cynic_verdicts",
        description = "List recent verdicts (limit defaults to 10)."
    )]
    async fn cynic_verdicts(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(10).to_string();
        self.get_query("/verdicts", &[("limit", limit)]).await
    }

    #[tool(
        name = "cynic_crystals",
        description = "List crystals (coherence patterns learned from verdict history)."
    )]
    async fn cynic_crystals(
        &self,
        params: Parameters<ListParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(20).to_string();
        self.get_query("/crystals", &[("limit", limit)]).await
    }

    #[tool(
        name = "cynic_audit_query",
        description = "Query MCP audit trail (tool calls, agent activity)."
    )]
    async fn cynic_audit_query(
        &self,
        params: Parameters<AuditQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(t) = p.tool {
            q.push(("tool", t));
        }
        if let Some(a) = p.agent_id {
            q.push(("agent_id", a));
        }
        let limit = p.limit.unwrap_or(20).to_string();
        q.push(("limit", limit));
        self.get_query("/audit", &q).await
    }

    #[tool(
        name = "cynic_library_query",
        description = "Query CYNIC's dynamic knowledge library from Kernel observations. Use for research, market signals, mechanism studies and source discoveries instead of repo-local Markdown notes."
    )]
    async fn cynic_library_query(
        &self,
        params: Parameters<LibraryQueryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(domain) = p.domain {
            q.push(("domain", domain));
        }
        if let Some(target) = p.target {
            q.push(("target", target));
        }
        if let Some(agent_id) = p.agent_id {
            q.push(("agent_id", agent_id));
        }
        if let Some(project) = p.project {
            q.push(("project", project));
        }
        if let Some(workstream) = p.workstream {
            q.push(("workstream", workstream));
        }
        if let Some(thread_id) = p.thread_id {
            q.push(("thread_id", thread_id));
        }
        let limit = p.limit.unwrap_or(20).min(100).to_string();
        q.push(("limit", limit));
        self.get_query("/observations", &q).await
    }

    #[tool(
        name = "cynic_metabolism",
        description = "Get the organism's metabolic state: observation intake, digestion ratio, crystallization rate, backlog, drops, verdict production. Zero-I/O snapshot of data flow health. Use to check if the organism is digesting its observations or starving."
    )]
    async fn cynic_metabolism(
        &self,
        _params: Parameters<MetabolismParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        // Metabolism is embedded in /health — extract it server-side.
        // We call /health and extract the metabolism + crystals + alerts subset.
        let resp = self
            .kernel
            .client()
            .get(self.kernel.url("/health"))
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());

        // Extract metabolism subset from full health response
        let extracted = match serde_json::from_str::<serde_json::Value>(&body) {
            Ok(health) => {
                serde_json::json!({
                    "metabolism": health.get("metabolism"),
                    "crystals": health.get("crystals"),
                    "alerts": health.get("alerts"),
                    "dogs": health.get("dogs").and_then(|d| d.as_array()).map(|dogs| {
                        dogs.iter().map(|d| serde_json::json!({
                            "id": d.get("id"),
                            "kind": d.get("kind"),
                            "circuit": d.get("circuit"),
                        })).collect::<Vec<_>>()
                    }),
                    "status": health.get("status"),
                    "uptime_seconds": health.get("uptime_seconds"),
                })
            }
            Err(_) => serde_json::json!({"raw": body}),
        };
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&extracted).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_compliance",
        description = "Query session compliance scores. With agent_id: score a specific agent's session (read-before-edit %, branch discipline, rule violations). Without: return recent compliance trend (last N sessions). Use to check if the organism is following its own rules."
    )]
    async fn cynic_compliance(
        &self,
        params: Parameters<ComplianceParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        if let Some(agent_id) = p.agent_id {
            // Per-session compliance
            self.get(&format!("/sessions/{agent_id}/compliance")).await
        } else {
            // Trend mode
            let limit = p.limit.unwrap_or(20).to_string();
            self.get_query("/compliance", &[("limit", limit)]).await
        }
    }

    #[tool(
        name = "cynic_infer",
        description = "Run sovereign local inference via llama-server (no cloud API, no quota). Forwarded to kernel's OpenAI-compatible endpoint. Use for tasks that should stay local: summarization, classification, extraction."
    )]
    async fn cynic_infer(
        &self,
        params: Parameters<InferParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_judge()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;

        if p.prompt.trim().is_empty() {
            return Err(McpError::invalid_params("prompt must not be empty", None));
        }

        // Build OpenAI-compatible chat completion request
        let mut messages = Vec::new();
        if let Some(sys) = &p.system {
            messages.push(serde_json::json!({"role": "system", "content": sys}));
        }
        messages.push(serde_json::json!({"role": "user", "content": p.prompt}));

        self.post(
            "/v1/chat/completions",
            &serde_json::json!({
                "messages": messages,
                "temperature": p.temperature.unwrap_or(0.7),
                "max_tokens": p.max_tokens.unwrap_or(512),
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_observe",
        description = "Record a tool observation (Edit, Write, Bash, etc.) for the Crystal Coherence Machine. Fire-and-forget."
    )]
    async fn cynic_observe(
        &self,
        params: Parameters<ObserveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let body = observe_request_body(&p);
        self.post("/observe", &body).await
    }

    #[tool(
        name = "cynic_dispatch_agent_task",
        description = "Dispatch a task for an agent (hermes, nightshift, etc.)."
    )]
    async fn cynic_dispatch_agent_task(
        &self,
        params: Parameters<DispatchAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        crate::types::validate_agent_task_content(&p.kind, &p.domain, &p.content)?;
        let kind = p.kind.clone();
        let domain = p.domain.clone();
        let body = dispatch_agent_task_body(&p);
        let response = self.post_json("/agent-tasks", &body).await?;
        let pending = self
            .get_query_json(
                "/agent-tasks",
                &[
                    ("kind", kind.clone()),
                    ("domain", domain.clone()),
                    ("limit", "50".to_string()),
                ],
            )
            .await?;
        let completed = self
            .get_query_json(
                "/agent-tasks/completed",
                &[
                    ("kind", kind.clone()),
                    ("domain", domain.clone()),
                    ("limit", "50".to_string()),
                ],
            )
            .await?;
        let result =
            verified_agent_task_dispatch_result(&response, &kind, &domain, &pending, &completed)
                .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()),
        )]))
    }

    #[tool(
        name = "cynic_list_pending_agent_tasks",
        description = "List pending tasks for a given agent kind."
    )]
    async fn cynic_list_pending_agent_tasks(
        &self,
        params: Parameters<ListPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(10).to_string();
        let mut query = vec![("kind", p.kind), ("limit", limit)];
        if let Some(domain) = p.domain {
            query.push(("domain", domain));
        }
        self.get_query("/agent-tasks", &query).await
    }

    #[tool(
        name = "cynic_watch_pending_agent_tasks",
        description = "Wait for pending tasks for a given agent kind through the kernel watch surface. Prefer this to repeated list polling when the consumer can block."
    )]
    async fn cynic_watch_pending_agent_tasks(
        &self,
        params: Parameters<WatchPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(10).to_string();
        let timeout_secs = p.timeout_secs.unwrap_or(30).clamp(1, 300).to_string();
        let mut query = vec![
            ("kind", p.kind),
            ("limit", limit),
            ("timeout_secs", timeout_secs),
        ];
        if let Some(domain) = p.domain {
            query.push(("domain", domain));
        }
        self.get_query("/agent-tasks/watch", &query).await
    }

    #[tool(
        name = "cynic_list_completed_agent_tasks",
        description = "List completed tasks for a given agent kind. Use after dispatch/claim/result flows to prove whether work was consumed rather than lost."
    )]
    async fn cynic_list_completed_agent_tasks(
        &self,
        params: Parameters<ListPendingAgentTasksParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let limit = p.limit.unwrap_or(10).to_string();
        let mut query = vec![("kind", p.kind), ("limit", limit)];
        if let Some(domain) = p.domain {
            query.push(("domain", domain));
        }
        self.get_query("/agent-tasks/completed", &query).await
    }

    #[tool(
        name = "cynic_claim_agent_task",
        description = "Atomically claim a pending agent task. Returns claimed=false when another agent already owns it."
    )]
    async fn cynic_claim_agent_task(
        &self,
        params: Parameters<ClaimAgentTaskParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        validate_agent_id(&p.agent_id)?;
        self.post(
            &format!("/agent-tasks/{}/claim", p.task_id),
            &serde_json::json!({
                "agent_id": p.agent_id.unwrap_or_else(|| "unknown".to_string()),
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_update_agent_task_result",
        description = "Update an agent task with its result or error."
    )]
    async fn cynic_update_agent_task_result(
        &self,
        params: Parameters<UpdateAgentTaskResultParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let body = serde_json::json!({
            "result": p.result,
            "error": p.error,
            "agent_id": p.agent_id,
        });
        let response = self
            .post_json(&format!("/agent-tasks/{}/result", p.task_id), &body)
            .await?;
        let Some(kind) = p.kind else {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "status": "completed",
                    "task_id": p.task_id,
                    "read_back_verified": false,
                    "read_back_required_fields": ["kind"],
                    "kernel_response": response,
                })
                .to_string(),
            )]));
        };
        let limit = p.limit.unwrap_or(50).to_string();
        let mut query = vec![("kind", kind.clone()), ("limit", limit)];
        if let Some(domain) = p.domain.clone() {
            query.push(("domain", domain));
        }
        let completed = self
            .get_query_json("/agent-tasks/completed", &query)
            .await?;
        let result = verified_agent_task_completion_result(
            &response,
            &p.task_id,
            &kind,
            p.domain.as_deref(),
            &completed,
        )
        .map_err(|e| McpError::internal_error(e, None))?;
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }
}
