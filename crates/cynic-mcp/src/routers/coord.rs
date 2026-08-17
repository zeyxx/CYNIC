// ── Coord tools (REST kernel) ───────────────────────────────

#[tool_router(router = tool_router_coord, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "cynic_agent_connection_status",
        description = "Read back the MCP-to-kernel agent connection, authentication authority, and registration state. Works while disconnected and never returns credentials."
    )]
    async fn cynic_agent_connection_status(
        &self,
        _params: Parameters<AgentConnectionStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let result = self.agent_connection_status().await;
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        )]))
    }

    #[tool(
        name = "cynic_coord_register",
        description = "Register this agent with the kernel coordination system."
    )]
    async fn cynic_coord_register(
        &self,
        params: Parameters<RegisterParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.require_mutation_contract()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/register",
            &serde_json::json!({
                "agent_id": agent_id,
                "intent": p.intent,
                "agent_type": p.agent_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_claim",
        description = "Claim a file or resource for exclusive editing."
    )]
    async fn cynic_coord_claim(
        &self,
        params: Parameters<ClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/claim",
            &serde_json::json!({
                "agent_id": agent_id,
                "target": p.target,
                "claim_type": p.claim_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_claim_batch",
        description = "Claim multiple files/resources at once."
    )]
    async fn cynic_coord_claim_batch(
        &self,
        params: Parameters<BatchClaimParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/claim-batch",
            &serde_json::json!({
                "agent_id": agent_id,
                "targets": p.targets,
                "claim_type": p.claim_type,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_renew",
        description = "Renew an active claim using its exact lease id and fencing token."
    )]
    async fn cynic_coord_renew(
        &self,
        params: Parameters<RenewLeaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/renew",
            &serde_json::json!({
                "agent_id": agent_id,
                "target": p.target,
                "lease_id": p.lease_id,
                "fencing_token": p.fencing_token,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_release",
        description = "Release a claim using target, lease_id, and fencing_token."
    )]
    async fn cynic_coord_release(
        &self,
        params: Parameters<ReleaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/release",
            &serde_json::json!({
                "agent_id": agent_id,
                "target": p.target,
                "lease_id": p.lease_id,
                "fencing_token": p.fencing_token,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_finalize",
        description = "Validate and atomically commit a runtime proof bundle while finalizing an active claim."
    )]
    async fn cynic_coord_finalize(
        &self,
        params: Parameters<FinalizeLeaseParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        let agent_id = self.coordination_principal()?;
        self.post(
            "/coord/finalize",
            &serde_json::json!({
                "agent_id": agent_id,
                "target": p.target,
                "lease_id": p.lease_id,
                "fencing_token": p.fencing_token,
                "proof_bundle": p.proof_bundle,
            }),
        )
        .await
    }

    #[tool(
        name = "cynic_coord_who",
        description = "Query active agents and their claims through the authenticated kernel coordination boundary."
    )]
    async fn cynic_coord_who(
        &self,
        params: Parameters<WhoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.rate_limit.check_other()?;
        let p = params.0;
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(a) = p.agent_id {
            q.push(("agent_id", a));
        }
        self.get_query("/coord/who", &q).await
    }

    #[tool(
        name = "coordination_contract_manifest",
        description = "Read the canonical coordination contract and optional client/project profiles used by consumers."
    )]
    async fn coordination_contract_manifest(
        &self,
        params: Parameters<CoordinationContractManifestParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;

        let protocol = load_coordination_protocol()?;
        let coordination_protocol = protocol
            .get("coordination_protocol")
            .cloned()
            .unwrap_or(protocol.clone());

        let permission_model = coordination_protocol
            .get("permission_model")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let clients = permission_model
            .get("profiles")
            .and_then(|value| value.get("clients"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let client_match = p.client.as_ref().and_then(|client| {
            clients
                .get(client)
                .cloned()
                .map(|c| serde_json::json!({client: c}))
        });

        let project_profile = if let Some(project) = &p.project {
            match load_consumer_profile(project)? {
                Some(profile) => Some(profile),
                None if p.include_default_project_profile.unwrap_or(false) => {
                    Some(serde_json::json!({"project": project}))
                }
                None => None,
            }
        } else {
            None
        };

        let operation = p.operation.as_deref();
        let operation_valid = if let Some(operation) = operation {
            coordination_protocol
                .get("capabilities")
                .and_then(|value| value.get("mcp_tools"))
                .and_then(|tools| tools.as_object())
                .map(|tools| {
                    tools.values().any(|tool_meta| {
                        tool_meta
                            .get("semantic")
                            .and_then(|semantic| semantic.as_str())
                            .is_some_and(|semantic_name| semantic_name == operation)
                    })
                })
                .unwrap_or(false)
        } else {
            true
        };

        if operation.is_some() && !operation_valid {
            return Err(McpError::invalid_params(
                format!("unsupported coordination operation: {operation:?}"),
                None,
            ));
        }

        let manifest = serde_json::json!({
            "coordination_protocol": coordination_protocol,
            "permission_model": permission_model,
            "query": {
                "client": p.client,
                "project": p.project,
                "operation": operation,
                "include_default_project_profile": p.include_default_project_profile,
            },
            "matches": {
                "client": client_match,
                "project_profile": project_profile,
            },
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| manifest.to_string()),
        )]))
    }

    #[tool(
        name = "coordination_status_publish",
        description = "Publish a structured coordination event (workstream/work item updates) to the CYNIC coordination domain."
    )]
    async fn coordination_status_publish(
        &self,
        params: Parameters<CoordinationStatusPublishParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        enforce_coordination_policy(&p.client, COORDINATION_OPERATION_STATUS_PUBLISH, &p.project)?;
        if !validate_coordination_event_name(&p.event) {
            return Err(McpError::invalid_params(
                "unsupported event; expected one of workstream.started|workstream.blocked|workstream.updated|interface.changed|run.completed|decision.recorded",
                None,
            ));
        }
        let mut validation_payload = serde_json::json!({
            "project": p.project,
            "workstream": p.workstream,
            "event": p.event,
            "summary": p.summary,
            "status": p.status.clone().unwrap_or_else(|| "ok".to_string()),
            "thread_id": p.thread_id,
            "branch": p.branch,
            "artifacts": p.artifacts,
            "open_points": p.open_points,
            "sensitivity": p.sensitivity,
        });
        if let Some(extra_payload) = p.payload.clone() {
            if let Some(extra_object) = extra_payload.as_object() {
                if let Some(object) = validation_payload.as_object_mut() {
                    object.extend(extra_object.clone());
                }
            }
        }
        validate_coordination_event_requirements(&p.event, &p.project, &validation_payload)?;
        let payload = emit_coordination_event_payload(
            "status.publish",
            &p.event,
            &p.project,
            &p.workstream,
            p.branch.as_deref(),
            p.thread_id.as_deref(),
            &p.summary,
            p.artifacts.clone(),
            p.open_points.clone(),
            serde_json::json!({
                "status": p.status,
                "payload": p.payload.unwrap_or_else(|| serde_json::json!({})),
                "sensitivity": p.sensitivity,
                "contract_version": p.contract_version,
            }),
        );
        let target =
            project_workstream_thread_key(&p.project, &p.workstream, &p.thread_id.as_deref());
        self.post(
            "/observe",
            &serde_json::json!({
                "tool": COORDINATION_TOOL,
                "target": target,
                "domain": COORDINATION_DOMAIN,
                "status": p.status.unwrap_or_else(|| "published".to_string()),
                "context": p.summary,
                "project": p.project,
                "tags": ["coordination","status","publish",p.event],
                "value": payload,
            }),
        )
        .await
    }

    #[tool(
        name = "coordination_status_read",
        description = "Read recent structured coordination events for a project/workstream/thread."
    )]
    async fn coordination_status_read(
        &self,
        params: Parameters<CoordinationStatusReadParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        enforce_coordination_policy(&p.client, COORDINATION_OPERATION_STATUS_READ, &p.project)?;
        let domain = p.domain.unwrap_or_else(|| COORDINATION_DOMAIN.to_string());
        let limit = p.limit.unwrap_or(50).to_string();
        let mut query = vec![
            ("domain", domain),
            ("project", p.project.clone()),
            ("limit", limit),
        ];
        if let Some(workstream) = &p.workstream {
            if !workstream.is_empty() {
                query.push(("workstream", workstream.clone()));
            }
        }
        if let Some(thread_id) = &p.thread_id {
            if !thread_id.is_empty() {
                query.push(("thread_id", thread_id.clone()));
            }
        }
        if let Some(workstream) = &p.workstream {
            if !workstream.is_empty() {
                let target =
                    project_workstream_thread_key(&p.project, workstream, &p.thread_id.as_deref());
                query.push(("target", target));
            }
        }
        let rows = self.get_query_json("/observations", &query).await?;
        let events: Vec<serde_json::Value> = rows
            .as_array()
            .into_iter()
            .flat_map(|items| items.iter())
            .filter(|row| coordination_filter_row(row, &p.project, &p.workstream, &p.thread_id))
            .map(normalize_coordination_row)
            .collect();
        Ok(coordination_tool_call_result(events))
    }

    #[tool(
        name = "coordination_thread_reply",
        description = "Post a reply payload into a known coordination thread."
    )]
    async fn coordination_thread_reply(
        &self,
        params: Parameters<CoordinationThreadReplyParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        enforce_coordination_policy(&p.client, COORDINATION_OPERATION_THREAD_REPLY, &p.project)?;
        let validation_payload = serde_json::json!({
            "project": p.project,
            "workstream": p.workstream,
            "thread_id": p.thread_id,
            "message": p.message,
            "branch": p.branch,
            "reply_to": p.reply_to,
            "artifacts": p.artifacts,
        });
        validate_coordination_event_requirements("thread.reply", &p.project, &validation_payload)?;
        let payload = emit_coordination_event_payload(
            "thread.reply",
            "thread.reply",
            &p.project,
            &p.workstream,
            p.branch.as_deref(),
            Some(p.thread_id.as_str()),
            &p.message,
            None,
            None,
            serde_json::json!({
                "reply_to": p.reply_to,
                "branch": p.branch,
            }),
        );
        let target =
            project_workstream_thread_key(&p.project, &p.workstream, &Some(p.thread_id.as_str()));
        self.post(
            "/observe",
            &serde_json::json!({
                "tool": COORDINATION_TOOL,
                "target": target,
                "domain": COORDINATION_DOMAIN,
                "status": "reply",
                "context": p.message,
                "project": p.project,
                "tags": ["coordination","thread","reply"],
                "value": payload,
            }),
        )
        .await
    }

    #[tool(
        name = "coordination_artifact_share",
        description = "Record an artifact share intent linked to a workstream/thread."
    )]
    async fn coordination_artifact_share(
        &self,
        params: Parameters<CoordinationArtifactShareParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        enforce_coordination_policy(&p.client, COORDINATION_OPERATION_ARTIFACT_SHARE, &p.project)?;
        if p.artifacts.is_empty() {
            return Err(McpError::invalid_params(
                "artifacts must not be empty",
                None,
            ));
        }
        let summary = if let Some(message) = p.message.clone() {
            message
        } else {
            format!(
                "Shared {} artifact(s) in {}",
                p.artifacts.len(),
                p.workstream
            )
        };
        let validation_payload = serde_json::json!({
            "project": p.project,
            "workstream": p.workstream,
            "summary": summary,
            "artifacts": p.artifacts,
            "thread_id": p.thread_id,
            "branch": p.branch,
        });
        validate_coordination_event_requirements(
            "artifact.share",
            &p.project,
            &validation_payload,
        )?;
        let payload = emit_coordination_event_payload(
            "artifact.share",
            "artifact.share",
            &p.project,
            &p.workstream,
            p.branch.as_deref(),
            p.thread_id.as_deref(),
            &summary,
            Some(p.artifacts.clone()),
            None,
            serde_json::json!({
                "branch": p.branch,
                "thread_id": p.thread_id,
            }),
        );
        let target =
            project_workstream_thread_key(&p.project, &p.workstream, &p.thread_id.as_deref());
        self.post(
            "/observe",
            &serde_json::json!({
                "tool": COORDINATION_TOOL,
                "target": target,
                "domain": COORDINATION_DOMAIN,
                "status": "artifact_shared",
                "context": summary,
                "project": p.project,
                "tags": ["coordination","artifact","share"],
                "value": payload,
            }),
        )
        .await
    }

    #[tool(
        name = "coordination_decision_record",
        description = "Record a repository-level decision with rationale and source artifact."
    )]
    async fn coordination_decision_record(
        &self,
        params: Parameters<CoordinationDecisionRecordParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let p = params.0;
        enforce_coordination_policy(
            &p.client,
            COORDINATION_OPERATION_DECISION_RECORD,
            &p.project,
        )?;
        let summary = format!("{} decided: {}", p.topic, p.decision);
        let validation_payload = serde_json::json!({
            "project": p.project,
            "workstream": p.workstream,
            "topic": p.topic,
            "decision": p.decision,
            "rationale": p.rationale,
            "source_artifact": p.source_artifact,
            "branch": p.branch,
            "thread_id": p.thread_id,
            "open_points": p.open_points,
            "artifacts": p.artifacts,
            "summary": summary,
        });
        validate_coordination_event_requirements(
            "decision.recorded",
            &p.project,
            &validation_payload,
        )?;
        let payload = emit_coordination_event_payload(
            "decision.record",
            "decision.record",
            &p.project,
            &p.workstream,
            p.branch.as_deref(),
            p.thread_id.as_deref(),
            &summary,
            Some(p.artifacts.clone().unwrap_or_default()),
            p.open_points.clone(),
            serde_json::json!({
                "topic": p.topic,
                "decision": p.decision,
                "rationale": p.rationale,
                "source_artifact": p.source_artifact,
                "branch": p.branch,
            }),
        );
        let target =
            project_workstream_thread_key(&p.project, &p.workstream, &p.thread_id.as_deref());
        self.post(
            "/observe",
            &serde_json::json!({
                "tool": COORDINATION_TOOL,
                "target": target,
                "domain": COORDINATION_DOMAIN,
                "status": "decision_recorded",
                "context": summary,
                "project": p.project,
                "tags": ["coordination","decision","record"],
                "value": payload,
            }),
        )
        .await
    }
}
