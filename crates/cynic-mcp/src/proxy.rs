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
use std::{borrow::Cow, fs, path::PathBuf, sync::Arc};

use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, service::MaybeSendFuture, tool, tool_handler,
    tool_router,
};

use crate::kernel_session::KernelSessionClient;
use crate::session_identity::SessionIdentity;
use crate::types::{
    AgentConnectionStatusParams, ApiContractsParams, AskesisLogParams, AttentionParams,
    AuditQueryParams, BatchClaimParams, ClaimAgentTaskParams, ClaimParams, ComplianceParams,
    DispatchAgentTaskParams, DockerContainerInspectParams, DockerContainerListParams,
    DockerContainerLogsParams, FederatedBootstrapStatusParams, FinalizeLeaseParams,
    FleetInventoryParams, FleetKernelCandidatesParams, FleetNodeExecParams, FleetNodeStatusParams,
    FleetReconcileApplyParams, FleetReconcilePlanParams, GitParams, IdentityAttestParams,
    IdentitySmokeCheckParams, InferParams, InferenceDiscoverParams, InferenceStartParams,
    InferenceSwitchParams, InfraProvisionParams,
    InfraRegistryReadParams, InfraRegistryValidateParams, JudgeParams, LibraryQueryParams,
    ListParams, ListPendingAgentTasksParams, LxcExecBridgeBundleParams, LxcExecBridgePlanParams,
    LxcExecBridgeStatusParams, McpRateLimit, MetabolismParams, NetworkIpamCheckParams,
    NetworkMeshDnsActivationApplyParams, NetworkMeshDnsActivationBundleParams,
    NetworkMeshDnsActivationPlanParams, NetworkMeshDnsActivationStatusParams,
    NetworkMeshDnsRenderParams, NetworkMeshProviderFallbackHostsParams,
    NetworkMeshProviderFallbackStatusParams, NetworkMeshStatusParams, ObserveParams,
    OperatorTransportStatusParams, ProviderControlStatusParams, RegisterParams, ReleaseParams,
    RenewLeaseParams, RuntimeReleaseDeclareParams, ServiceHealthParams, ServiceStatusParams,
    SourceControlParams, SshTargetInspectParams, StoragePrimaryBundleParams,
    StoragePrimaryPlanParams, StoragePrimaryReadinessParams, StoragePrimaryStatusParams,
    SourceChangeSubmitParams, UpdateAgentTaskResultParams, ValidateParams, WatchPendingAgentTasksParams, WhoParams,
    validate_agent_id,
};
use cynic_host::ports::argv_execution::{
    ArgvExecutionPort, ArgvExecutionRequest, LocalArgvExecutionAdapter,
};

#[cfg(feature = "legacy_infra_tools")]
use crate::types::{ProxmoxActionParams, ProxmoxExecParams, ProxmoxProvisionParams};

type Content = rmcp::model::ContentBlock;

fn project_root() -> PathBuf {
    crate::project_root::resolve_mcp_project_root()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LifecycleDispatchParams {
    pub profile: String,
}

include!("helpers.rs");

// ── Proxy struct ────────────────────────────────────────────

#[derive(Clone)]
pub struct CynicMcpProxy {
    kernel: KernelSessionClient,
    client: reqwest::Client,
    argv_execution: Arc<dyn ArgvExecutionPort>,
    rate_limit: Arc<McpRateLimit>,
    project_root: String,
    session_identity: SessionIdentity,
    // WHY: K12 — tool_router is used via trait dispatch, compiler misses direct usage
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl std::fmt::Debug for CynicMcpProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CynicMcpProxy")
            .field("kernel", &self.kernel)
            .finish_non_exhaustive()
    }
}

impl CynicMcpProxy {
    #[cfg(feature = "legacy_infra_tools")]
    #[allow(dead_code)]
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
            + Self::tool_router_local_fleet()
            + Self::tool_router_local_operator_transport()
            + Self::tool_router_local_agent_surfaces()
            + Self::tool_router_local_runtime_release()
            + Self::tool_router_local_infra_registry()
            + Self::tool_router_local_network()
            + Self::tool_router_local_fleet_execution()
            + Self::tool_router_local_services()
            + Self::tool_router_local_solana()
            + Self::tool_router_local_storage()
            + Self::tool_router_local_legacy_proxmox()
            + Self::tool_router_local_lifecycle_inference()
            + Self::tool_router_eval()
            + Self::tool_router_agent_loop_audit()
            + Self::tool_router_docker()
    }

    pub fn tool_router_forward() -> ToolRouter<Self> {
        Self::tool_router_forward_contracts()
            + Self::tool_router_forward_health()
            + Self::tool_router_forward_infer()
            + Self::tool_router_forward_legacy_tasks()
            + Self::tool_router_forward_observe()
    }

    pub fn new(
        kernel: KernelSessionClient,
        session_identity: SessionIdentity,
        project_root: String,
    ) -> Self {
        Self::with_execution_port(
            kernel,
            session_identity,
            project_root,
            Arc::new(LocalArgvExecutionAdapter),
        )
    }

    pub fn with_execution_port(
        kernel: KernelSessionClient,
        session_identity: SessionIdentity,
        project_root: String,
        argv_execution: Arc<dyn ArgvExecutionPort>,
    ) -> Self {
        // WHY: reqwest::Client::builder().build() only fails on TLS init error,
        // which is unrecoverable. Unwrap is safe here but clippy wants map_err.
        #[allow(clippy::expect_used)] // WHY: TLS init failure = unrecoverable
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("HTTP client init failed");

        Self {
            kernel,
            client,
            argv_execution,
            rate_limit: Arc::new(McpRateLimit::new()),
            project_root,
            session_identity,
            tool_router: Self::tool_router(),
        }
    }

    fn require_auth(&self) -> Result<(), McpError> {
        if !self.kernel.is_active() {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32000),
                "MCP agent session is expired or unavailable",
                None,
            ));
        }
        Ok(())
    }

    /// Admit a mutation only when this MCP artifact proves compatibility with
    /// the desired agent contract.  Read-only observation and plan projection
    /// deliberately remain available while reconciliation is blocked.
    fn require_mutation_contract(&self) -> Result<(), McpError> {
        let plan = crate::runtime_contract::observe_runtime_contract(
            std::path::Path::new(&self.project_root),
        )
        .map_err(|error| {
            McpError::new(
                rmcp::model::ErrorCode(-32001),
                format!("MCP runtime contract is unobservable: {error}"),
                None,
            )
        })?;
        if !crate::runtime_contract::admits_mutation(&plan) {
            return Err(McpError::new(
                rmcp::model::ErrorCode(-32001),
                format!(
                    "MCP runtime contract blocks mutation: status={:?}; reconcile the admitted runtime artifact and read back its contract before retrying",
                    plan.status
                ),
                None,
            ));
        }
        Ok(())
    }

    fn coordination_principal(&self) -> Result<String, McpError> {
        let agent_id = self.session_identity.agent_id.clone();
        validate_agent_id(&Some(agent_id.clone()))?;
        Ok(agent_id)
    }
}

include!("runtime_helpers.rs");
include!("kernel_forward.rs");
include!("routers/identity.rs");
include!("routers/forward.rs");
include!("routers/coord.rs");
include!("routers/local.rs");
include!("routers/eval.rs");
include!("routers/docker.rs");
include!("handler.rs");
