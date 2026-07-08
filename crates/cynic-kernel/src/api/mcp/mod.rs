//! CYNIC MCP Server — Model Context Protocol interface for AI agents.
//!
//! Exposes CYNIC kernel capabilities as MCP tools consumable by
//! Claude Code, Gemini CLI, Hermes Agent, or any MCP-compatible client.
//!
//! Architecture: thin wrappers around existing domain logic (Judge, StoragePort).
//! Zero duplication with REST handlers — both call the same core.
//!
//! Tools split by concern:
//! - judge_tools.rs: auth, judge, health, verdicts, crystals,  audit
//! - coord_tools.rs: register, claim, claim_batch, release, who
//! - observe_tools.rs: observe, validate, git

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter, model::*, tool_handler,
};
use schemars::JsonSchema;
use serde::Deserialize;

mod agent_tools;
mod coord_tools;
mod observe_tools;
pub mod proxy;

use crate::domain::coord::CoordPort;
use crate::domain::events::KernelEvent;
use crate::domain::storage::StoragePort;

// ── MCP Rate Limiter ──────────────────────────────────────────

pub(crate) struct McpRateLimit {
    judge_calls: AtomicU64,
    other_calls: AtomicU64,
    window_start: std::sync::Mutex<std::time::Instant>,
}

impl McpRateLimit {
    fn new() -> Self {
        Self {
            judge_calls: AtomicU64::new(0),
            other_calls: AtomicU64::new(0),
            window_start: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    pub(crate) fn check_judge(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.judge_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 10 {
            self.judge_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 10 judge/infer calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn check_other(&self) -> Result<(), McpError> {
        self.maybe_reset();
        let count = self.other_calls.fetch_add(1, Ordering::Relaxed);
        if count >= 30 {
            self.other_calls.fetch_sub(1, Ordering::Relaxed);
            Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Rate limit: max 30 calls per minute",
                None,
            ))
        } else {
            Ok(())
        }
    }

    fn maybe_reset(&self) {
        if let Ok(mut start) = self.window_start.lock()
            && start.elapsed() > std::time::Duration::from_secs(60)
        {
            self.judge_calls.store(0, Ordering::Relaxed);
            self.other_calls.store(0, Ordering::Relaxed);
            *start = std::time::Instant::now();
        }
    }
}

// ── Input Validation Helpers ──────────────────────────────────

pub(crate) fn validate_agent_id(agent_id: &Option<String>) -> Result<(), McpError> {
    if let Some(id) = agent_id {
        crate::domain::coord::validate_agent_id(id)
            .map_err(|msg| McpError::invalid_params(msg, None))?;
    }
    Ok(())
}

pub(crate) fn validate_agent_task_content(
    kind: &str,
    domain: &str,
    content: &str,
) -> Result<(), McpError> {
    if kind != "hermes" || !matches!(domain, "organ-forge" | "repo-lifecycle") {
        return Ok(());
    }
    let parsed: serde_json::Value = serde_json::from_str(content).map_err(|e| {
        McpError::invalid_params(
            format!("agent task content must be valid JSON for {domain}: {e}"),
            None,
        )
    })?;
    let has_declared_targets = ["coord_targets", "targets", "repo_targets", "files"]
        .iter()
        .any(|key| {
            parsed
                .get(key)
                .and_then(|value| value.as_array())
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        });
    if !has_declared_targets {
        return Err(McpError::invalid_params(
            "repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files",
            None,
        ));
    }
    validate_forge_lifecycle_contract(&parsed, load_registry().as_ref())
        .map_err(|message| McpError::invalid_params(message, None))?;
    Ok(())
}

fn validate_forge_lifecycle_contract(
    parsed: &serde_json::Value,
    registry: Option<&serde_json::Value>,
) -> Result<(), String> {
    let lifecycle_name = required_non_empty_string(parsed, "lifecycle")?;
    let contract_fields = [
        "orchestrator_organ",
        "build_runner_role",
        "deploy_target_role",
        "build_target",
        "deploy_target",
    ];
    for field in contract_fields {
        required_non_empty_string(parsed, field)?;
    }

    let Some(registry) = registry else {
        return Ok(());
    };
    let Some(lifecycle) = registry
        .get("lifecycle")
        .and_then(|value| value.get(lifecycle_name))
    else {
        return Ok(());
    };

    require_registry_match(parsed, lifecycle, "orchestrator_organ")?;
    require_registry_match(parsed, lifecycle, "build_runner_role")?;
    require_registry_match(parsed, lifecycle, "deploy_target_role")?;
    require_registry_json_match(parsed, lifecycle, "runtime_paths")?;
    require_registry_json_match(parsed, lifecycle, "runtime_path_proofs")?;
    require_registry_json_match(parsed, lifecycle, "activation_evidence_contract")?;

    let nodes = registry
        .get("nodes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let build_role = lifecycle
        .get("build_runner_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_role = lifecycle
        .get("deploy_target_role")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let deploy_flag = lifecycle
        .get("deploy_target_flag")
        .and_then(|value| value.as_str())
        .unwrap_or("deploy_target");
    if let Some(expected_build_target) = node_name_for_role(&nodes, build_role, None) {
        require_content_value(parsed, "build_target", &expected_build_target)?;
    }
    if let Some(expected_deploy_target) = node_name_for_role(&nodes, deploy_role, Some(deploy_flag))
    {
        require_content_value(parsed, "deploy_target", &expected_deploy_target)?;
    }
    Ok(())
}

fn required_non_empty_string<'a>(
    parsed: &'a serde_json::Value,
    field: &str,
) -> Result<&'a str, String> {
    let value = parsed
        .get(field)
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if value.is_empty() {
        return Err(format!("repo-affecting forge tasks must declare {field}"));
    }
    Ok(value)
}

fn require_registry_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field).and_then(|value| value.as_str()) {
        require_content_value(parsed, field, expected)?;
    }
    Ok(())
}

fn require_registry_json_match(
    parsed: &serde_json::Value,
    lifecycle: &serde_json::Value,
    field: &str,
) -> Result<(), String> {
    if let Some(expected) = lifecycle.get(field)
        && parsed.get(field) != Some(expected)
    {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}"
        ));
    }
    Ok(())
}

fn require_content_value(
    parsed: &serde_json::Value,
    field: &str,
    expected: &str,
) -> Result<(), String> {
    if parsed.get(field).and_then(|value| value.as_str()) != Some(expected) {
        return Err(format!(
            "repo-affecting forge tasks must match registry {field}={expected}"
        ));
    }
    Ok(())
}

fn node_name_for_role(
    nodes: &[serde_json::Value],
    role: &str,
    deploy_flag: Option<&str>,
) -> Option<String> {
    nodes
        .iter()
        .find(|node| {
            node.get("role").and_then(|value| value.as_str()) == Some(role)
                && deploy_flag
                    .map(|flag| node.get(flag).and_then(|value| value.as_bool()) == Some(true))
                    .unwrap_or(true)
        })
        .and_then(|node| node.get("name").and_then(|value| value.as_str()))
        .map(String::from)
}

fn load_registry() -> Option<serde_json::Value> {
    let project_root = match std::env::var("CYNIC_PROJECT_ROOT") {
        Ok(path) => Some(std::path::PathBuf::from(path)),
        Err(env_error) => {
            tracing::warn!(error = %env_error, "CYNIC_PROJECT_ROOT not set; resolving registry path from git");
            match std::process::Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output()
            {
                Ok(output) => match String::from_utf8(output.stdout) {
                    Ok(path) => Some(std::path::PathBuf::from(path.trim())),
                    Err(utf8_error) => {
                        tracing::warn!(error = %utf8_error, "git top-level path was not valid UTF-8");
                        None
                    }
                },
                Err(git_error) => {
                    tracing::warn!(error = %git_error, "failed to resolve git top-level path");
                    match std::env::current_dir() {
                        Ok(path) => Some(path),
                        Err(cwd_error) => {
                            tracing::warn!(error = %cwd_error, "failed to resolve current directory for registry path");
                            None
                        }
                    }
                }
            }
        }
    };
    let registry_path = project_root?.join("infra/registry.json");
    let content = match std::fs::read_to_string(&registry_path) {
        Ok(content) => content,
        Err(read_error) => {
            tracing::warn!(error = %read_error, path = %registry_path.display(), "failed to read registry.json");
            return None;
        }
    };
    match serde_json::from_str(&content) {
        Ok(registry) => Some(registry),
        Err(parse_error) => {
            tracing::warn!(error = %parse_error, path = %registry_path.display(), "failed to parse registry.json");
            None
        }
    }
}

pub(crate) fn sanitize_error(category: &str) -> McpError {
    McpError::internal_error(format!("{category} unavailable — check /health"), None)
}

// ── MCP Tool Parameters ─────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JudgeParams {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    pub dogs: Option<Vec<String>>,
    pub agent_id: Option<String>,
    pub crystals: Option<bool>,
    /// Optional: sensitivity level. "high" forces routing to sovereign (local) Dogs only.
    /// Use for private content: DMs, wallet seeds, API keys. Default: none (auto-detected).
    pub sensitivity: Option<String>,
    /// Optional: slot acquisition priority. Controls queuing behavior on sovereign inference.
    /// Values: "user" (30s wait, default), "hermes" (15s), "nightshift" (skip if busy), "background" (skip).
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListParams {
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InferParams {
    pub system: Option<String>,
    pub prompt: String,
    pub agent_id: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuditQueryParams {
    pub tool: Option<String>,
    pub agent_id: Option<String>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AuthParams {
    pub api_key: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterParams {
    pub agent_id: String,
    pub intent: String,
    pub agent_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimParams {
    pub agent_id: String,
    pub target: String,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchClaimParams {
    pub agent_id: String,
    pub targets: Vec<String>,
    pub claim_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReleaseParams {
    pub agent_id: String,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhoParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScopeParams {
    pub agent_id: String,
    pub scope: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetabolismParams {
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComplianceParams {
    pub agent_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ObserveParams {
    pub tool: String,
    pub target: Option<String>,
    pub domain: Option<String>,
    pub status: Option<String>,
    pub context: Option<String>,
    pub project: Option<String>,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DispatchAgentTaskParams {
    pub kind: String,   // "hermes" | "nightshift" | future agent
    pub domain: String, // "twitter" | "token" | "on-chain"
    /// JSON task payload. Repo-affecting hermes tasks must declare coord_targets, targets, repo_targets, or files.
    pub content: String,
    pub agent_id: Option<String>,
    pub due_at: Option<String>,
    pub priority: Option<i64>,
    pub consumer: Option<String>,
    pub escalation_policy: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPendingAgentTasksParams {
    pub kind: String,
    pub domain: Option<String>,
    pub limit: Option<u32>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClaimAgentTaskParams {
    pub task_id: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateAgentTaskResultParams {
    pub task_id: String,
    pub result: Option<String>,
    pub error: Option<String>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CrystalObserveParams {
    /// Crystal ID to observe. If omitted, auto-generated from content hash.
    pub crystal_id: Option<String>,
    /// The content being crystallized.
    pub content: String,
    /// Domain label (e.g. "chess", "token", "general").
    pub domain: String,
    /// φ-bounded score [0.0, 0.618].
    pub score: f64,
    /// Source identifier (e.g. "hermes", "nightshift", "claude-mcp").
    pub source: String,
    /// Optional sentiment: "positive" | "negative" | "neutral".
    pub sentiment: Option<String>,
    pub agent_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CrystalShatterParams {
    /// Crystal ID to shatter (instant dissolution).
    pub crystal_id: String,
    /// Human-readable reason for shattering.
    pub reason: String,
    /// Source identifier (e.g. "cortex", "hermes", "maintenance").
    pub source: String,
    pub agent_id: String,
}

// ── MCP Server ───────────────────────────────────────────────

#[derive(Clone)]
    // WHY: Dormant logic kept for Phase 2, disabled as part of Option B deep purge
#[allow(dead_code)]
pub struct CynicMcp {
    
    pub(crate) storage: Arc<dyn StoragePort>,
    pub(crate) coord: Arc<dyn CoordPort>,
    
    pub(crate) embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
    
    
    pub(crate) metrics: Arc<crate::domain::metrics::Metrics>,
    pub(crate) environment:
        Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    pub(crate) task_health: Arc<crate::infra::task_health::TaskHealth>,
    pub(crate) system_contract: crate::domain::contract::SystemContract,
    pub(crate) event_tx: Option<tokio::sync::broadcast::Sender<KernelEvent>>,
    pub(crate) rate_limit: Arc<McpRateLimit>,
    pub(crate) bg_semaphore: Arc<tokio::sync::Semaphore>,
    pub(crate) authenticated: Arc<AtomicBool>,
    
    
    
    
    
    // WHY: K12 — tool_router is used via trait dispatch, compiler misses direct usage
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcp").finish_non_exhaustive()
    }
}

impl CynicMcp {
    pub fn tool_router() -> ToolRouter<Self> {
        Self::tool_router_coord()
            + Self::tool_router_observe()
            + Self::tool_router_agent()
    }

    #[allow(clippy::too_many_arguments)]
    // WHY: Constructor receives many kernel dependencies — each is a distinct port required at the MCP
    // surface. A builder pattern would only rename the argument list; called in exactly one place.
    pub fn new(
        
        storage: Arc<dyn StoragePort>,
        coord: Arc<dyn CoordPort>,
        
        embedding: Arc<dyn crate::domain::embedding::EmbeddingPort>,
        
        
        metrics: Arc<crate::domain::metrics::Metrics>,
        environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
        task_health: Arc<crate::infra::task_health::TaskHealth>,
        system_contract: crate::domain::contract::SystemContract,
        event_tx: Option<tokio::sync::broadcast::Sender<KernelEvent>>,
        
        
        
        
        
    ) -> Self {
        Self {
            
            storage,
            coord,
            embedding,
            
            
            metrics,
            environment,
            task_health,
            system_contract,
            
            event_tx,
            
            
            
            
            
            rate_limit: Arc::new(McpRateLimit::new()),
            bg_semaphore: Arc::new(tokio::sync::Semaphore::new(
                crate::domain::constants::BG_SEMAPHORE_PERMITS,
            )),
            // Auto-auth when CYNIC_API_KEY is set. MCP over stdio = local subprocess.
            authenticated: Arc::new(AtomicBool::new(
                !std::env::var("CYNIC_API_KEY")
                    .unwrap_or_default()
                    .is_empty()
                    || std::env::var("CYNIC_ALLOW_OPEN_API")
                        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                        .unwrap_or(false),
            )),
            tool_router: Self::tool_router(),
        }
    }

    pub(crate) fn require_auth(&self) -> Result<(), McpError> {
        if !self.authenticated.load(Ordering::Relaxed) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "Not authenticated — call cynic_auth first",
                None,
            ));
        }
        Ok(())
    }

    pub(crate) async fn touch(&self, agent_id: &str) {
        if !agent_id.is_empty()
            && agent_id != "unknown"
            && self.coord.heartbeat(agent_id).await.is_err()
        {}
    }

    pub(crate) async fn audit(&self, tool_name: &str, agent_id: &str, details: &serde_json::Value) {
        if let Err(e) = self
            .coord
            .store_audit(tool_name, agent_id, &details.to_string())
            .await
        {
            tracing::debug!(error = %e, tool_name, "MCP audit store failed (non-fatal)");
        }
        self.touch(agent_id).await;
    }
}

#[tool_handler]
impl ServerHandler for CynicMcp {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("cynic-kernel", env!("CYNIC_VERSION")))
            .with_instructions("CYNIC epistemic immune system — independent AI validators reaching consensus under mathematical doubt. φ-bounded at 61.8%.")
    }
}

