#[tool_router(router = tool_router_identity, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "identity_attest",
        description = "Return typed runtime identity presence, class, and provenance without exposing values, locators, shapes, lengths, or commitments."
    )]
    async fn identity_attest(
        &self,
        _params: Parameters<IdentityAttestParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let identities = [
            cynic_host::secret_presence::observe_runtime_secret_presence(
                &cynic_host::secret_presence::RuntimeSecretPresenceSpec::storage_runtime(),
            ),
            cynic_host::secret_presence::observe_runtime_secret_presence(
                &cynic_host::secret_presence::RuntimeSecretPresenceSpec::proxmox_control(),
            ),
        ];
        let provider_control_process =
            cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::
                observe_process_configuration();
        let all_known_present = identities.iter().all(|identity| identity.present);
        let provider_control_process_configured = provider_control_process.endpoint
            && provider_control_process.client_identity
            && provider_control_process.credential;
        let process_identity_ready = all_known_present && provider_control_process_configured;
        let credential_lease =
            cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::
                observe_runtime_credential_lease();
        let result = serde_json::json!({
            "schema": "cynic.identity.attestation.v2",
            "status": if process_identity_ready { "process_identity_ready" } else { "process_identity_incomplete" },
            "scope": "mcp_process",
            "authority": "local_process_environment_and_secret_presence",
            "all_known_present": all_known_present,
            "identities": identities,
            "provider_control_process": provider_control_process,
            "provider_control_process_configured": provider_control_process_configured,
            "process_identity_ready": process_identity_ready,
            "credential_lease": credential_lease,
            "secret_exposed": false,
            "next_action": "observe provider_control_status and node enrollment separately; process identity does not prove provider enrollment",
        });
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }

    #[tool(
        name = "identity_smoke_check",
        description = "Check credential validity against live provider APIs without exposing raw values."
    )]
    async fn identity_smoke_check(
        &self,
        params: Parameters<IdentitySmokeCheckParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let providers = params
            .0
            .providers
            .unwrap_or_else(|| vec!["porkbun".to_string(), "cloudflare".to_string()]);
        let mut results = Vec::new();
        for provider in providers {
            match provider.as_str() {
                "porkbun" => results.push(self.porkbun_identity_smoke_check().await?),
                "cloudflare" => results.push(self.cloudflare_identity_smoke_check().await?),
                "proxmox" => {
                    let status = cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::observe_from_env(
                        std::time::Duration::from_secs(10),
                    )
                    .await;
                    results.push(serde_json::json!({
                        "provider": "proxmox",
                        "ok": status.status == "ready",
                        "status": status,
                        "secret_exposed": false,
                    }));
                }
                other => results.push(serde_json::json!({
                    "provider": other,
                    "ok": false,
                    "message": "Unsupported provider",
                })),
            }
        }
        let all_ok = results.iter().all(|item| {
            item.get("ok")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        });
        Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "op": "identity_smoke_check",
                "all_ok": all_ok,
                "results": results,
            })
            .to_string(),
        )]))
    }

    async fn porkbun_identity_smoke_check(&self) -> Result<serde_json::Value, McpError> {
        let api_key = std::env::var("PORKBUN_API_KEY").unwrap_or_default();
        let secret_key = std::env::var("PORKBUN_SECRET_API_KEY").unwrap_or_default();
        if api_key.is_empty() || secret_key.is_empty() {
            return Ok(serde_json::json!({
                "provider": "porkbun",
                "ok": false,
                "message": "Missing: PORKBUN_API_KEY or PORKBUN_SECRET_API_KEY",
            }));
        }
        let response = self
            .client
            .post("https://api.porkbun.com/api/json/v3/ping")
            .json(&serde_json::json!({ "apikey": api_key, "secretapikey": secret_key }))
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("Porkbun ping failed: {error}"), None)
            })?;
        let payload: serde_json::Value = response.json().await.unwrap_or_default();
        let ok = payload.get("status").and_then(|value| value.as_str()) == Some("SUCCESS");
        Ok(serde_json::json!({
            "provider": "porkbun",
            "ok": ok,
            "message": if ok { "Porkbun ping OK".to_string() } else {
                format!("Porkbun ping failed: {}", payload.get("message").and_then(|value| value.as_str()).unwrap_or("unknown"))
            },
        }))
    }

    async fn cloudflare_identity_smoke_check(&self) -> Result<serde_json::Value, McpError> {
        let token = std::env::var("CF_API_TOKEN").unwrap_or_default();
        if token.is_empty() {
            return Ok(serde_json::json!({
                "provider": "cloudflare",
                "ok": false,
                "message": "Missing: CF_API_TOKEN",
            }));
        }
        let response = self
            .client
            .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|error| {
                McpError::internal_error(format!("Cloudflare verify failed: {error}"), None)
            })?;
        let payload: serde_json::Value = response.json().await.unwrap_or_default();
        let ok = payload
            .get("success")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            && payload
                .pointer("/result/status")
                .and_then(|value| value.as_str())
                == Some("active");
        Ok(serde_json::json!({
            "provider": "cloudflare",
            "ok": ok,
            "message": if ok { "Cloudflare token active" } else { "Cloudflare token invalid" },
        }))
    }
}
