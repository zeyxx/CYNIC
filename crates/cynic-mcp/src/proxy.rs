//! MCP-to-REST proxy — thin forwarder that replaces the full kernel in MCP mode.
//!
//! Instead of spawning Judge+Storage+Dogs+Metrics locally, forwards tool calls
//! to the running REST kernel via HTTP. Zero local state, zero SurrealDB connection.
//!
//! Local tools (validate, git) run in-process (filesystem access required).
//! All other tools → HTTP to REST kernel.

#![allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::redundant_clone,
    clippy::map_err_ignore,
    clippy::unwrap_used
)]

use schemars::JsonSchema;
use serde::Deserialize;
use sha3::{Digest, Sha3_512};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};

use crate::types::{
    AttentionParams, AuditQueryParams, AuthParams, BatchClaimParams, ClaimAgentTaskParams,
    ClaimParams, ComplianceParams, CoordinationArtifactShareParams,
    CoordinationContractManifestParams, CoordinationDecisionRecordParams,
    CoordinationStatusPublishParams, CoordinationStatusReadParams, CoordinationThreadReplyParams,
    DispatchAgentTaskParams, FleetNodeExecParams, FleetNodeStatusParams, GitParams,
    IdentityAttestParams, IdentitySmokeCheckParams, InferParams, InfraProvisionParams,
    InfraRegistryReadParams, InfraRegistryValidateParams, JudgeParams, LibraryQueryParams,
    ListParams, ListPendingAgentTasksParams, LxcExecBridgeBundleParams, LxcExecBridgePlanParams,
    LxcExecBridgeStatusParams, McpRateLimit, MetabolismParams, NetworkIpamCheckParams,
    NetworkMeshDnsActivationBundleParams, NetworkMeshDnsActivationPlanParams,
    NetworkMeshDnsActivationStatusParams, NetworkMeshDnsRenderParams,
    NetworkMeshProviderFallbackHostsParams, NetworkMeshProviderFallbackStatusParams,
    NetworkMeshStatusParams, ObserveParams, ProxmoxActionParams, ProxmoxExecParams,
    ProxmoxProvisionParams, RegisterParams, ReleaseParams, ServiceHealthParams,
    ServiceStatusParams, StoragePrimaryBundleParams, StoragePrimaryPlanParams,
    StoragePrimaryReadinessParams, StoragePrimaryStatusParams, UpdateAgentTaskResultParams,
    ValidateParams, WatchPendingAgentTasksParams, WhoParams, validate_agent_id,
    validate_agent_task_content,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LifecycleDispatchParams {
    pub profile: String,
}

include!("helpers.rs");

// ── Proxy struct ────────────────────────────────────────────

#[derive(Clone)]
pub struct CynicMcpProxy {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    authenticated: Arc<AtomicBool>,
    rate_limit: Arc<McpRateLimit>,
    project_root: String,
    // WHY: K12 — tool_router is used via trait dispatch, compiler misses direct usage
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcpProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcpProxy")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl CynicMcpProxy {
    fn proxmox_credential() -> String {
        std::env::var("CYNIC_PROXMOX_TOKEN_SECRET")
            .or_else(|_| std::env::var("CYNIC_PROXMOX_SECRET"))
            .map(|value| value.trim().to_string())
            .unwrap_or_default()
    }

    pub fn tool_router() -> ToolRouter<Self> {
        Self::tool_router_forward()
            + Self::tool_router_coord()
            + Self::tool_router_identity()
            + Self::tool_router_local()
    }

    pub fn new(base_url: String, api_key: String, project_root: String) -> Self {
        // WHY: reqwest::Client::builder().build() only fails on TLS init error,
        // which is unrecoverable. Unwrap is safe here but clippy wants map_err.
        #[allow(clippy::expect_used)] // WHY: TLS init failure = unrecoverable
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client init failed");

        let auto_auth = !api_key.is_empty()
            || std::env::var("CYNIC_ALLOW_OPEN_API")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false);

        Self {
            client,
            base_url,
            api_key,
            // MCP stdio is a local subprocess; if the wrapper injected the kernel token,
            // requiring the model to echo it back through cynic_auth adds exposure only.
            authenticated: Arc::new(AtomicBool::new(auto_auth)),
            rate_limit: Arc::new(McpRateLimit::new()),
            project_root,
            tool_router: Self::tool_router(),
        }
    }

    fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "MCP not authenticated: CYNIC_API_KEY was not provided to the MCP process",
                None,
            ));
        }
        Ok(())
    }

    async fn proxmox_cluster_resources(&self) -> serde_json::Value {
        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();
        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return serde_json::json!({
                "status": "unavailable",
                "reason": "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars"
            });
        }

        let client = match reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
        {
            Ok(client) => client,
            Err(e) => {
                return serde_json::json!({
                    "status": "error",
                    "reason": format!("Proxmox client error: {e}")
                });
            }
        };
        let auth_header = format!("PVEAPIToken={client_id}={credential_value}");
        match client
            .get(format!("{proxmox_url}/api2/json/cluster/resources?type=vm"))
            .header("Authorization", auth_header)
            .send()
            .await
        {
            Ok(response) => match response.json::<serde_json::Value>().await {
                Ok(payload) => serde_json::json!({
                    "status": "ok",
                    "resources": payload.get("data").cloned().unwrap_or_else(|| serde_json::json!([]))
                }),
                Err(e) => serde_json::json!({
                    "status": "error",
                    "reason": format!("Invalid JSON from Proxmox: {e}")
                }),
            },
            Err(e) => serde_json::json!({
                "status": "error",
                "reason": format!("Proxmox API failed: {e}")
            }),
        }
    }

    async fn proxmox_qemu_agent_exec(
        &self,
        node: &str,
        vmid: &str,
        command: &str,
        timeout_seconds: u64,
    ) -> Result<serde_json::Value, McpError> {
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
            .timeout(std::time::Duration::from_secs(timeout_seconds.max(5)))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_header = format!("PVEAPIToken={client_id}={credential_value}");
        let exec_url = format!("{proxmox_url}/api2/json/nodes/{node}/qemu/{vmid}/agent/exec");
        let form = vec![
            ("command", "sh".to_string()),
            ("command", "-lc".to_string()),
            ("command", command.to_string()),
        ];

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

        let status_url =
            format!("{proxmox_url}/api2/json/nodes/{node}/qemu/{vmid}/agent/exec-status?pid={pid}");
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(timeout_seconds.max(1)) {
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
                return Ok(serde_json::json!({
                    "status": "completed",
                    "provider": "proxmox_qemu_guest_agent",
                    "node": node,
                    "vmid": vmid,
                    "pid": pid,
                    "exit_code": status_json["data"]["exitcode"].as_i64().unwrap_or(-1),
                    "stdout": status_json["data"]["out-data"].as_str().unwrap_or(""),
                    "stderr": status_json["data"]["err-data"].as_str().unwrap_or(""),
                }));
            }
        }

        Ok(serde_json::json!({
            "status": "timeout",
            "provider": "proxmox_qemu_guest_agent",
            "node": node,
            "vmid": vmid,
            "pid": pid,
            "timeout_seconds": timeout_seconds,
        }))
    }

    fn lxc_exec_bridge_config() -> Option<(String, String)> {
        let ssh_target = std::env::var("CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())?;
        let helper = std::env::var("CYNIC_LXC_EXEC_BRIDGE_HELPER")
            .unwrap_or_else(|_| "/usr/local/libexec/cynic-lxc-exec-bridge".to_string());
        Some((ssh_target, helper))
    }

    fn run_lxc_exec_bridge(
        ssh_target: &str,
        helper: &str,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let mut child = std::process::Command::new("ssh")
            .args([
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                ssh_target,
                "sudo",
                "-n",
                helper,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| McpError::internal_error(format!("LXC bridge SSH failed: {e}"), None))?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(
                    serde_json::to_string(request)
                        .unwrap_or_default()
                        .as_bytes(),
                )
                .map_err(|e| {
                    McpError::internal_error(format!("LXC bridge stdin write failed: {e}"), None)
                })?;
        }
        let output = child.wait_with_output().map_err(|e| {
            McpError::internal_error(format!("LXC bridge SSH wait failed: {e}"), None)
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let parsed = serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|_| {
            serde_json::json!({
                "schema": "cynic.lxc_exec_bridge.response.v1",
                "status": "error",
                "error": "bridge_output_not_json",
                "stdout": stdout,
                "stderr": stderr,
                "ssh_exit_code": output.status.code()
            })
        });
        Ok(serde_json::json!({
            "status": if output.status.success() { "completed" } else { "bridge_failed" },
            "provider": "lxc_exec_bridge_ssh",
            "ssh_target": ssh_target,
            "helper": helper,
            "ssh_exit_code": output.status.code(),
            "bridge": parsed,
            "stderr": stderr,
        }))
    }

    fn run_host_exec_transport(
        argv: &[String],
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let Some(program) = argv.first() else {
            return Err(McpError::internal_error(
                "host exec transport command is empty",
                None,
            ));
        };
        let mut child = std::process::Command::new(program)
            .args(&argv[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                McpError::internal_error(format!("host exec transport spawn failed: {e}"), None)
            })?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(
                    serde_json::to_string(request)
                        .unwrap_or_default()
                        .as_bytes(),
                )
                .map_err(|e| {
                    McpError::internal_error(
                        format!("host exec transport stdin write failed: {e}"),
                        None,
                    )
                })?;
        }
        let output = child.wait_with_output().map_err(|e| {
            McpError::internal_error(format!("host exec transport wait failed: {e}"), None)
        })?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let parsed = serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or_else(|_| {
            serde_json::json!({
                "schema": "cynic.host_exec_transport.response.v1",
                "status": "error",
                "error": "transport_output_not_json",
                "stdout": stdout,
                "stderr": stderr,
                "exit_code": output.status.code()
            })
        });
        Ok(serde_json::json!({
            "status": if output.status.success() { "completed" } else { "transport_failed" },
            "provider": "host_exec_transport",
            "command": argv,
            "exit_code": output.status.code(),
            "transport": parsed,
            "stderr": stderr,
        }))
    }

    // ── HTTP helpers ────────────────────────────────────────

    async fn get(&self, path: &str) -> Result<CallToolResult, McpError> {
        let resp = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    async fn get_query(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<CallToolResult, McpError> {
        let url = query_url(&self.base_url, path, params)?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    async fn get_query_json(
        &self,
        path: &str,
        params: &[(&str, String)],
    ) -> Result<serde_json::Value, McpError> {
        let url = query_url(&self.base_url, path, params)?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("REST kernel GET {path} failed with HTTP {status}: {text}"),
                None,
            ));
        }
        serde_json::from_str(&text).map_err(|e| {
            McpError::internal_error(format!("REST kernel returned invalid JSON: {e}"), None)
        })
    }

    async fn post(&self, path: &str, body: &serde_json::Value) -> Result<CallToolResult, McpError> {
        let resp = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    async fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let resp = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("REST kernel unreachable: {e}"), None))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| r#"{"error":"response read failed"}"#.into());
        if !status.is_success() {
            return Err(McpError::internal_error(
                format!("REST kernel POST {path} failed with HTTP {status}: {text}"),
                None,
            ));
        }
        serde_json::from_str(&text).map_err(|e| {
            McpError::internal_error(format!("REST kernel returned invalid JSON: {e}"), None)
        })
    }
}

#[tool_router(router = tool_router_identity, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[tool(
        name = "identity_attest",
        description = "Return credential presence and commitments from the local env membrane without exposing raw values."
    )]
    async fn identity_attest(
        &self,
        _params: Parameters<IdentityAttestParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        let env_file = std::env::var("CYNIC_ENV_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                    .join(".cynic-env")
            });
        let content = fs::read_to_string(&env_file).unwrap_or_default();
        let mut entries = Vec::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let stripped = trimmed.strip_prefix("export ").unwrap_or(trimmed);
            let Some((key, value)) = stripped.split_once('=') else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }
            let value = value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            let shape = if value.len() <= 4 {
                "****".to_string()
            } else {
                format!(
                    "{}{}",
                    &value[..4],
                    "*".repeat(std::cmp::min(8, value.len().saturating_sub(4)))
                )
            };
            entries.push(serde_json::json!({
                "key": key,
                "present": true,
                "shape": shape,
                "length": value.len(),
                "commitment": format!("{:x}", Sha3_512::digest(value.as_bytes())),
                "commitment_schema": "cynic.identity.commitment.v2",
                "source": env_file.display().to_string(),
            }));
        }
        let result = serde_json::json!({
            "schema": "cynic.identity.attestation.v1",
            "source": env_file.display().to_string(),
            "all_known_present": !entries.is_empty(),
            "groups": {
                "env": entries,
            },
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
                "porkbun" => {
                    let api_key = std::env::var("PORKBUN_API_KEY").unwrap_or_default();
                    let secret_key = std::env::var("PORKBUN_SECRET_API_KEY").unwrap_or_default();
                    if api_key.is_empty() || secret_key.is_empty() {
                        results.push(serde_json::json!({
                            "provider": "porkbun",
                            "ok": false,
                            "message": "Missing: PORKBUN_API_KEY or PORKBUN_SECRET_API_KEY",
                        }));
                        continue;
                    }
                    let body = serde_json::json!({
                        "apikey": api_key,
                        "secretapikey": secret_key,
                    });
                    let response = self
                        .client
                        .post("https://api.porkbun.com/api/json/v3/ping")
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| {
                            McpError::internal_error(format!("Porkbun ping failed: {e}"), None)
                        })?;
                    let payload: serde_json::Value = response
                        .json()
                        .await
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let ok =
                        payload.get("status").and_then(|value| value.as_str()) == Some("SUCCESS");
                    results.push(serde_json::json!({
                        "provider": "porkbun",
                        "ok": ok,
                        "message": if ok { "Porkbun ping OK".to_string() } else { format!("Porkbun ping failed: {}", payload.get("message").and_then(|value| value.as_str()).unwrap_or("unknown")) },
                    }));
                }
                "cloudflare" => {
                    let token = std::env::var("CF_API_TOKEN").unwrap_or_default();
                    if token.is_empty() {
                        results.push(serde_json::json!({
                            "provider": "cloudflare",
                            "ok": false,
                            "message": "Missing: CF_API_TOKEN",
                        }));
                        continue;
                    }
                    let response = self
                        .client
                        .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
                        .bearer_auth(token)
                        .send()
                        .await
                        .map_err(|e| {
                            McpError::internal_error(format!("Cloudflare verify failed: {e}"), None)
                        })?;
                    let payload: serde_json::Value = response
                        .json()
                        .await
                        .unwrap_or_else(|_| serde_json::json!({}));
                    let ok = payload
                        .get("success")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                        && payload
                            .get("result")
                            .and_then(|value| value.get("status"))
                            .and_then(|value| value.as_str())
                            == Some("active");
                    results.push(serde_json::json!({
                        "provider": "cloudflare",
                        "ok": ok,
                        "message": if ok { "Cloudflare token active" } else { "Cloudflare token invalid" },
                    }));
                }
                other => {
                    results.push(serde_json::json!({
                        "provider": other,
                        "ok": false,
                        "message": "Unsupported provider",
                    }));
                }
            }
        }
        let all_ok = results
            .iter()
            .all(|item| item.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let result = serde_json::json!({
            "op": "identity_smoke_check",
            "all_ok": all_ok,
            "results": results,
        });
        Ok(CallToolResult::success(vec![Content::text(
            result.to_string(),
        )]))
    }
}

include!("routers/forward.rs");
include!("routers/coord.rs");
include!("routers/local.rs");
include!("handler.rs");
