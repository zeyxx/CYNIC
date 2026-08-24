//! REST API type definitions — request/response structs and shared state.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::Mutex;

use crate::domain::agent_session::AgentSessionRuntime;
use crate::domain::coord::CoordPort;
use crate::domain::events::KernelEvent;
use crate::domain::mail::MailPort;
use crate::domain::metrics::Metrics;
use crate::domain::node_control::NodeControlRuntime;
use crate::domain::node_enrollment::NodeEnrollmentRuntime;
use crate::domain::organ::OrganPort;
use crate::domain::storage::StoragePort;
use crate::domain::usage::DogUsageTracker;
use crate::domain::verdict_cache::VerdictCache;
use crate::domain::wisdom::DomainCurations;
use crate::infra::task_health::TaskHealth;
use crate::introspection::Alert;
use cynic_inference::embedding::EmbeddingPort;
use cynic_inference::{SlotSemaphoreMap, SlotTracker};

// ── ROLE-SCOPED AUTH ──────────────────────────────────────
// Three roles match the organism's communication patterns:
// - CORTEX: human reasoning sessions (Claude, Gemini) — can judge
// - ORGAN: sensors and limbs (Hermes, organ-X) — can observe, not judge
// - INTERNAL: kernel↔kernel (Nightshift, MCP proxy) — unrestricted

/// Organism role — attached to every authenticated request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Human reasoning sessions — can /judge, /observe, read everything
    Cortex,
    /// Organs and sensors — can /observe and /coord only
    Organ,
    /// Kernel-to-kernel — unrestricted (Nightshift, MCP proxy, admin)
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCapability {
    CoordinationRead,
    CoordinationWrite,
    ObservationWrite,
    InferenceUse,
    RuntimeRead,
    AgentWorkloadDelivery,
    JudgmentInvoke,
    /// Federation websocket ingress (`/events/ws`). Organ/Internal only —
    /// modelled here so the WS route consumes the same canonical decision
    /// source as REST instead of an inline role check.
    FederationEvents,
    RuntimeAdmin,
}

pub fn authorize_role_request(role: Role, method: &str, path: &str) -> bool {
    let required = required_capability(method, path);
    match role {
        Role::Internal => true,
        Role::Cortex => !matches!(required, RuntimeCapability::FederationEvents),
        Role::Organ => matches!(
            required,
            RuntimeCapability::CoordinationRead
                | RuntimeCapability::CoordinationWrite
                | RuntimeCapability::ObservationWrite
                | RuntimeCapability::InferenceUse
                | RuntimeCapability::RuntimeRead
                | RuntimeCapability::FederationEvents
        ),
    }
}

pub(crate) fn required_capability(method: &str, path: &str) -> RuntimeCapability {
    if path.starts_with("/coord/") {
        return if method == "GET" {
            RuntimeCapability::CoordinationRead
        } else {
            RuntimeCapability::CoordinationWrite
        };
    }
    if path.starts_with("/node-control/") {
        if path.ends_with("/agent-workloads") || path.contains("/agent-workloads/") {
            RuntimeCapability::AgentWorkloadDelivery
        } else if method == "GET" {
            RuntimeCapability::RuntimeRead
        } else {
            RuntimeCapability::CoordinationWrite
        }
    } else if path == "/agent-workloads" || path.starts_with("/agent-workloads/") {
        RuntimeCapability::AgentWorkloadDelivery
    } else if path == "/observe" || (path.starts_with("/crystals/") && path.ends_with("/observe")) {
        RuntimeCapability::RuntimeRead
    } else if path == "/proof-of-history" {
        RuntimeCapability::RuntimeRead
    } else if path.starts_with("/v1/") {
        RuntimeCapability::InferenceUse
    } else if matches!(path, "/health" | "/events") {
        RuntimeCapability::RuntimeRead
    } else if path == "/events/ws" {
        RuntimeCapability::FederationEvents
    } else if path == "/judge" {
        RuntimeCapability::JudgmentInvoke
    } else {
        RuntimeCapability::RuntimeAdmin
    }
}

pub(crate) fn authorize_agent_session_request(
    record: &cynic_protocol::AgentSessionRecord,
    method: &str,
    path: &str,
) -> bool {
    let (id, mutates) = match required_capability(method, path) {
        RuntimeCapability::CoordinationRead => ("coord.read", false),
        RuntimeCapability::CoordinationWrite => ("coord.write", true),
        RuntimeCapability::ObservationWrite => ("observation.write", true),
        RuntimeCapability::InferenceUse => ("inference.use", false),
        RuntimeCapability::RuntimeRead => ("runtime.read", false),
        RuntimeCapability::AgentWorkloadDelivery => ("agent_workload.delivery", method != "GET"),
        RuntimeCapability::JudgmentInvoke => ("judgment.invoke", true),
        // Agent-session capability grants never include federation websocket
        // ingress; that plane is signed-workload principal only.
        RuntimeCapability::FederationEvents => ("federation.events", true),
        RuntimeCapability::RuntimeAdmin => ("runtime.admin", true),
    };
    record
        .capabilities
        .iter()
        .any(|grant| grant.id == id && (!mutates || grant.mutates))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationMechanism {
    LocalAgentSession,
    AgentBearer,
    SharedBearer,
    BoundEd25519,
    WalletSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedPrincipal {
    pub subject: Option<String>,
    pub role: Role,
    pub mechanism: AuthenticationMechanism,
}

impl AuthenticatedPrincipal {
    pub fn require_agent(&self, asserted_agent_id: &str) -> Result<(), &'static str> {
        if !matches!(
            self.mechanism,
            AuthenticationMechanism::LocalAgentSession
                | AuthenticationMechanism::AgentBearer
                | AuthenticationMechanism::BoundEd25519
        ) {
            return Err("coordination mutation requires an agent-bound workload identity");
        }
        match self.subject.as_deref() {
            Some(subject) if subject == asserted_agent_id => Ok(()),
            Some(_) => Err("authenticated principal does not own the asserted agent_id"),
            None => Err("coordination mutation requires an agent-bound credential"),
        }
    }
}

pub struct SigningPrincipalBinding {
    pub subject: String,
    pub role: Role,
    public_key: String,
}

impl SigningPrincipalBinding {
    pub fn new(subject: String, role: Role, public_key: String) -> Result<Self, String> {
        crate::domain::coord::validate_agent_id(&subject)
            .map_err(|error| format!("invalid signing principal subject `{subject}`: {error}"))?;
        if public_key.len() != 64 || !public_key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "signing principal `{subject}` must declare one Ed25519 public key as 64 hexadecimal characters"
            ));
        }
        Ok(Self {
            subject,
            role,
            public_key: public_key.to_ascii_lowercase(),
        })
    }
}

impl std::fmt::Debug for SigningPrincipalBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SigningPrincipalBinding")
            .field("subject", &self.subject)
            .field("role", &self.role)
            .field(
                "public_key_fingerprint",
                &format_args!("{}…", &self.public_key[..12]),
            )
            .finish()
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cortex => write!(f, "cortex"),
            Self::Organ => write!(f, "organ"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

/// Authentication authority loaded at boot.
///
/// Bound signing principals are the preferred workload path. Bearer slots are
/// compatibility inputs and must not be treated as durable workload identity.
pub struct AgentCredentialBinding {
    pub subject: String,
    pub role: Role,
    token: String,
}

impl AgentCredentialBinding {
    pub fn new(subject: String, role: Role, token: String) -> Result<Self, String> {
        crate::domain::coord::validate_agent_id(&subject)
            .map_err(|error| format!("invalid agent credential subject `{subject}`: {error}"))?;
        if token.is_empty() {
            return Err(format!("agent credential `{subject}` has an empty token"));
        }
        Ok(Self {
            subject,
            role,
            token,
        })
    }
}

impl std::fmt::Debug for AgentCredentialBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCredentialBinding")
            .field("subject", &self.subject)
            .field("role", &self.role)
            .field("has_token", &!self.token.is_empty())
            .finish()
    }
}

pub struct RoleKeyMap {
    pub cortex: Option<String>,
    pub organ: Option<String>,
    pub internal: Option<String>,
    pub legacy: Option<String>,
    pub agent_bindings: Vec<AgentCredentialBinding>,
    pub signing_bindings: Vec<SigningPrincipalBinding>,
}

impl std::fmt::Debug for RoleKeyMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoleKeyMap")
            .field("has_cortex", &self.cortex.is_some())
            .field("has_organ", &self.organ.is_some())
            .field("has_internal", &self.internal.is_some())
            .field("has_legacy", &self.legacy.is_some())
            .field("agent_bindings", &self.agent_bindings)
            .field("signing_bindings", &self.signing_bindings)
            .finish()
    }
}

impl RoleKeyMap {
    pub fn from_environment(legacy: Option<String>) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct AgentCredentialSpec {
            token: String,
            role: String,
        }

        #[derive(Deserialize)]
        struct SigningPrincipalSpec {
            public_key: String,
            role: String,
        }

        let agent_bindings = match std::env::var("CYNIC_AGENT_KEYS_JSON") {
            Ok(raw) if !raw.trim().is_empty() => {
                let specs: HashMap<String, AgentCredentialSpec> = serde_json::from_str(&raw)
                    .map_err(|error| format!("invalid CYNIC_AGENT_KEYS_JSON: {error}"))?;
                let mut bindings = Vec::with_capacity(specs.len());
                for (subject, spec) in specs {
                    let role = match spec.role.as_str() {
                        "cortex" => Role::Cortex,
                        "organ" => Role::Organ,
                        "internal" => Role::Internal,
                        _ => {
                            return Err(format!(
                                "agent credential `{subject}` has unsupported role `{}`",
                                spec.role
                            ));
                        }
                    };
                    if bindings.iter().any(|binding: &AgentCredentialBinding| {
                        super::middleware::constant_time_eq(
                            binding.token.as_bytes(),
                            spec.token.as_bytes(),
                        )
                    }) {
                        return Err("agent credential tokens must be unique".into());
                    }
                    bindings.push(AgentCredentialBinding::new(subject, role, spec.token)?);
                }
                bindings
            }
            _ => Vec::new(),
        };

        let signing_bindings = match std::env::var("CYNIC_SIGNING_PRINCIPALS_JSON") {
            Ok(raw) if !raw.trim().is_empty() => {
                let specs: HashMap<String, SigningPrincipalSpec> = serde_json::from_str(&raw)
                    .map_err(|error| format!("invalid CYNIC_SIGNING_PRINCIPALS_JSON: {error}"))?;
                let mut bindings = Vec::with_capacity(specs.len());
                for (subject, spec) in specs {
                    let role = parse_role(&subject, &spec.role)?;
                    let binding = SigningPrincipalBinding::new(subject, role, spec.public_key)?;
                    if bindings.iter().any(|candidate: &SigningPrincipalBinding| {
                        candidate.public_key == binding.public_key
                    }) {
                        return Err("signing principal public keys must be unique".into());
                    }
                    bindings.push(binding);
                }
                bindings
            }
            _ => Vec::new(),
        };

        Ok(Self {
            cortex: std::env::var("CYNIC_API_KEY_CORTEX")
                .ok()
                .filter(|value| !value.is_empty()),
            organ: std::env::var("CYNIC_API_KEY_ORGAN")
                .ok()
                .filter(|value| !value.is_empty()),
            internal: std::env::var("CYNIC_API_KEY_INTERNAL")
                .ok()
                .filter(|value| !value.is_empty()),
            legacy,
            agent_bindings,
            signing_bindings,
        })
    }

    /// Match a bearer token to an authenticated principal. Returns None if no match.
    /// Checks all slots with constant-time comparison — no early exit on match
    /// to prevent timing leaks about which role a token belongs to.
    pub fn resolve(&self, token: &str) -> Option<AuthenticatedPrincipal> {
        let token_bytes = token.as_bytes();
        let mut result: Option<Role> = None;

        // Check each slot — constant-time per slot, but we check all slots
        // regardless of match to avoid timing side-channels.
        if self
            .internal
            .as_ref()
            .is_some_and(|k| super::middleware::constant_time_eq(token_bytes, k.as_bytes()))
        {
            result = Some(Role::Internal);
        }
        if self
            .cortex
            .as_ref()
            .is_some_and(|k| super::middleware::constant_time_eq(token_bytes, k.as_bytes()))
        {
            result = Some(Role::Cortex);
        }
        if self
            .organ
            .as_ref()
            .is_some_and(|k| super::middleware::constant_time_eq(token_bytes, k.as_bytes()))
        {
            result = Some(Role::Organ);
        }
        // Legacy fallback: old CYNIC_API_KEY grants Cortex role
        if result.is_none()
            && self
                .legacy
                .as_ref()
                .is_some_and(|k| super::middleware::constant_time_eq(token_bytes, k.as_bytes()))
        {
            result = Some(Role::Cortex);
        }
        let mut principal = result.map(|role| AuthenticatedPrincipal {
            subject: None,
            role,
            mechanism: AuthenticationMechanism::SharedBearer,
        });
        for binding in &self.agent_bindings {
            if super::middleware::constant_time_eq(token_bytes, binding.token.as_bytes()) {
                principal = Some(AuthenticatedPrincipal {
                    subject: Some(binding.subject.clone()),
                    role: binding.role,
                    mechanism: AuthenticationMechanism::AgentBearer,
                });
            }
        }
        principal
    }

    pub fn resolve_signer(&self, public_key: &str) -> Option<AuthenticatedPrincipal> {
        let normalized = public_key.to_ascii_lowercase();
        self.signing_bindings
            .iter()
            .find(|binding| {
                super::middleware::constant_time_eq(
                    normalized.as_bytes(),
                    binding.public_key.as_bytes(),
                )
            })
            .map(|binding| AuthenticatedPrincipal {
                subject: Some(binding.subject.clone()),
                role: binding.role,
                mechanism: AuthenticationMechanism::BoundEd25519,
            })
    }

    pub fn has_any_authentication_authority(&self) -> bool {
        self.cortex.is_some()
            || self.organ.is_some()
            || self.internal.is_some()
            || self.legacy.is_some()
            || !self.agent_bindings.is_empty()
            || !self.signing_bindings.is_empty()
    }
}

fn parse_role(subject: &str, role: &str) -> Result<Role, String> {
    match role {
        "cortex" => Ok(Role::Cortex),
        "organ" => Ok(Role::Organ),
        "internal" => Ok(Role::Internal),
        _ => Err(format!(
            "principal `{subject}` has unsupported role `{role}`"
        )),
    }
}

#[cfg(test)]
mod authorization_tests {
    use super::{Role, RuntimeCapability, authorize_role_request, required_capability};

    #[test]
    fn organ_role_is_bounded_by_the_canonical_capability_mapping() {
        assert!(authorize_role_request(Role::Organ, "POST", "/observe"));
        assert!(authorize_role_request(Role::Organ, "GET", "/coord/who"));
        assert!(authorize_role_request(
            Role::Organ,
            "POST",
            "/coord/register"
        ));
        assert!(!authorize_role_request(Role::Organ, "POST", "/judge"));
        assert!(!authorize_role_request(
            Role::Organ,
            "POST",
            "/lifecycle/upgrade"
        ));
    }

    #[test]
    fn agent_workload_delivery_does_not_require_runtime_admin() {
        assert_eq!(
            required_capability("POST", "/agent-workloads"),
            RuntimeCapability::AgentWorkloadDelivery
        );
        assert_eq!(
            required_capability("GET", "/agent-workloads/build-123"),
            RuntimeCapability::AgentWorkloadDelivery
        );
    }

    #[test]
    fn federation_ws_route_uses_canonical_decision_source() {
        assert_eq!(
            required_capability("GET", "/events/ws"),
            RuntimeCapability::FederationEvents
        );
        assert!(authorize_role_request(Role::Organ, "GET", "/events/ws"));
        assert!(authorize_role_request(Role::Internal, "GET", "/events/ws"));
        // Cortex is unrestricted on REST but excluded from federation
        // websocket ingress; the boundary lives here, not inline in the
        // websocket handler.
        assert!(!authorize_role_request(Role::Cortex, "GET", "/events/ws"));
    }
}

const LEGACY_DOG_ID: &str = "gemini-cli";
const CANONICAL_DOG_ID: &str = "antigravity-cli";

// ── PUSH-PRODUCER HEARTBEAT ────────────────────────────────

/// Tracks the last /observe POST from a push-based producer.
/// Pull senses have OrganPort::freshness(). This covers push producers.
/// K15 consumer: introspection alerts on silent permanent producers.
#[derive(Debug, Clone)]
pub struct ProducerHeartbeat {
    pub last_seen: std::time::Instant,
    pub count: u64,
}

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub agent_sessions: Option<Arc<AgentSessionRuntime>>,
    pub node_control: Option<Arc<NodeControlRuntime>>,
    pub node_enrollment: Option<Arc<NodeEnrollmentRuntime>>,
    pub storage: Arc<dyn StoragePort>,
    pub coord: Arc<dyn CoordPort>,
    pub embedding: Arc<dyn EmbeddingPort>,
    pub usage: Arc<Mutex<DogUsageTracker>>,
    pub verdict_cache: Arc<VerdictCache>,
    pub task_health: Arc<TaskHealth>,
    pub metrics: Arc<Metrics>,
    pub api_key: Option<String>,
    pub role_keys: RoleKeyMap,
    pub storage_info: StorageInfo,
    pub rate_limiter: PerIpRateLimiter,
    pub judge_limiter: PerIpRateLimiter,
    /// F22: Cached /ready result — avoids DB ping on every probe call.
    /// Updated by readiness_handler when stale (>30s).
    pub ready_cache: ReadyCache,
    /// Bounds fire-and-forget background spawns (observe, audit).
    /// Prevents unbounded task accumulation under DB degradation.
    pub bg_semaphore: Arc<tokio::sync::Semaphore>,
    /// Tracks fire-and-forget spawns for drain at shutdown.
    /// Semaphore bounds concurrency; TaskTracker enables wait-for-completion.
    pub bg_tasks: tokio_util::task::TaskTracker,
    /// Latest introspection alerts (updated every 5min by background task).
    /// Empty = healthy system. RwLock: read-heavy (every /health), write every 5min.
    pub introspection_alerts: Arc<std::sync::RwLock<Vec<Alert>>>,
    /// F23: Bounds concurrent SSE connections to prevent FD exhaustion.
    /// 32 connections is generous for operational monitoring.
    pub sse_semaphore: Arc<tokio::sync::Semaphore>,
    /// Kernel event bus — broadcast to all SSE/WebSocket subscribers.
    /// Capacity 256: events are small, subscribers should keep up.
    /// Lagging subscribers get BroadcastStreamRecvError::Lagged → skip.
    pub event_tx: tokio::sync::broadcast::Sender<KernelEvent>,
    /// Boot-time integrity verification: did the last stored verdict's BLAKE3 hash match?
    /// Set once at boot, exposed in /health for proprioception.
    pub chain_verified: AtomicBool,
    /// Latest proprioceptive snapshot — updated by probe scheduler (Ring 2).
    /// None until first probe cycle completes. Auth-gated in /health.
    pub environment: Arc<std::sync::RwLock<Option<crate::domain::probe::EnvironmentSnapshot>>>,
    /// Tracks TTL for dynamically registered Dogs (via POST /dogs/register).
    /// Config-based Dogs are permanent and not tracked here.
    /// RwLock: read on every heartbeat, write on register/expire.
    pub registered_dogs: Arc<std::sync::RwLock<HashMap<String, RegisteredDog>>>,
    /// OS-level service process table — tracks all registered organism services.
    /// Services announce via POST /services/register; TTL-evicted on silence.
    /// This is the kernel's equivalent of /proc — desired state = registry.json,
    /// observed state = this map. Delta = what's missing or unknown.
    pub registered_services: Arc<std::sync::RwLock<HashMap<String, RegisteredService>>>,
    /// D4: In-memory store for async judge jobs — progressive Dog arrival polling.
    /// Self-model: expected system state loaded from backends.toml at boot.
    /// Compared against live roster in /health to detect missing Dogs.
    pub system_contract: Arc<std::sync::RwLock<crate::domain::contract::SystemContract>>,
    /// Expected system backends config loaded at boot.
    pub backend_configs: Vec<crate::infra::config::BackendConfig>,
    /// Token enricher — fetches on-chain data for domain=token-analysis stimuli.
    /// None if Helius API key not configured. Graceful degradation: Dogs judge raw address.
    pub enricher: Option<Arc<dyn crate::domain::enrichment::TokenEnricherPort>>,
    /// Sensory organ readers — organism perceiving external data stores.
    /// K15 deferred-consumer: deadline 2026-05-26. If no consumer by then, remove.
    pub senses: Vec<Arc<dyn OrganPort>>,
    /// Curated domain signals (D1-D6) loaded at boot for wisdom enrichment.
    /// K15 consumer: dogs fetch matching patterns from curations to enrich stimulus.
    pub domain_curations: Arc<DomainCurations>,
    /// Domain-aware Dog router — selects suitable Dogs based on domain hint.
    /// Initialized from backend_configs at boot. Enables data-centric Dog selection.
    pub domain_router: Arc<crate::infra::domain_router::DomainRouter>,
    /// Dynamic routing calculator — selects Dogs based on observed latencies.
    /// Consumes dog_performance observations and adapts routing in real-time.
    pub routing_calc: Arc<crate::infra::routing_calc::RoutingCalculator>,
    /// Dog performance collector — aggregates latency/success metrics per Dog.
    /// K15 seam 3 producer: on_dog callbacks feed observations here.
    /// Periodically flushed to routing_calc for live routing adaptation.
    pub dog_perf_collector: Arc<crate::infra::dog_performance::DogPerformanceCollector>,
    /// Soma L2: Slot semaphore map — per-Dog permit tracking for slot coordination.
    /// Initialized by the health loop at boot; Judge::evaluate acquires/releases permits.
    pub slot_semaphores: Arc<SlotSemaphoreMap>,
    /// Soma L2: Slot utilization tracker — updated by health loop every 30s.
    /// Consumers: /judge (skip saturated Dogs), /health (expose utilization),
    /// GET /inference/slots (external dispatch query).
    pub slot_tracker: Arc<SlotTracker>,
    /// Soma L4: Inference proxy routing table — maps dog_id → upstream llama-server URL.
    pub proxy_targets: Arc<cynic_inference::application::proxy_routing::ProxyTargets>,
    /// Mail service — organis email operations (inbox, send, sync).
    /// None if mail backend not configured.
    pub mail: Option<Arc<dyn MailPort>>,
    /// Phase 3: Node registry for WebSocket federation.
    /// Tracks connected nodes with TTL-based eviction (120s default).
    pub node_registry: Arc<crate::api::websocket::NodeRegistry>,
    /// Zone config — maps file paths to ownership zones for dispatch visibility.
    /// Loaded from .cortex/zones.json at boot. Read-only after init.
    pub zones: Arc<crate::domain::zones::ZoneConfig>,
    /// Push-producer heartbeat registry — tracks last /observe POST per agent_id.
    /// K15 consumer: introspection alerts when permanent producers go silent.
    /// Absence of signal becomes signal of absence.
    pub producer_heartbeats: Arc<std::sync::RwLock<HashMap<String, ProducerHeartbeat>>>,
    /// Deduplicates critical overdue-task observations emitted from health/introspection paths.
    pub overdue_task_alerted: Arc<std::sync::RwLock<HashMap<String, String>>>,
}

/// Storage topology — exposed on authenticated /health for discoverability.
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub namespace: String,
    pub database: String,
}

/// F22: Cached DB readiness — avoids storage.ping() on every /ready call.
/// 30s TTL. Under load, /ready called every 5-10s by probes.
pub struct ReadyCache {
    ok: AtomicBool,
    checked_at: AtomicU64,
}

impl Default for ReadyCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadyCache {
    const TTL_SECS: u64 = 30;

    pub fn new() -> Self {
        Self {
            ok: AtomicBool::new(false),
            checked_at: AtomicU64::new(0),
        }
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Returns cached value if fresh, None if stale.
    pub fn get(&self) -> Option<bool> {
        let age = Self::now_secs().saturating_sub(self.checked_at.load(Ordering::Acquire));
        if age <= Self::TTL_SECS {
            Some(self.ok.load(Ordering::Acquire))
        } else {
            None
        }
    }

    pub fn set(&self, ok: bool) {
        self.ok.store(ok, Ordering::Release);
        self.checked_at.store(Self::now_secs(), Ordering::Release);
    }
}

impl std::fmt::Debug for ReadyCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReadyCache")
            .field("ok", &self.ok.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("storage_info", &self.storage_info)
            .field("api_key", &self.api_key.as_ref().map(|_| "***"))
            .finish_non_exhaustive()
    }
}

impl AppState {
    pub fn storage_metrics(&self) -> Option<crate::domain::storage::StorageMetrics> {
        self.storage.metrics()
    }

    /// Centralized system health assessment (SSoT for REST and MCP).
    /// Computes status from Dogs, storage, probes, background tasks, and contract.
    pub async fn system_health(&self) -> crate::domain::health_gate::HealthAssessment {
        let dog_health: Vec<(String, String, u32)> = vec![];
        let (healthy_dogs, total_dogs) =
            crate::domain::health_gate::count_healthy_dogs(&dog_health);

        let storage_ok = self.storage.ping().await.is_ok();
        let live_dog_ids = vec![];

        let contract_delta = {
            let contract_guard = self
                .system_contract
                .read()
                .unwrap_or_else(|e| e.into_inner());
            contract_guard.assess(&live_dog_ids)
        };

        let probes_degraded =
            crate::domain::probe::EnvironmentSnapshot::is_degraded(&self.environment);
        let stale_tasks = self.task_health.readiness_stale_tasks();
        let overdue_task_ids: Vec<String> = if storage_ok {
            self.storage
                .list_overdue_agent_tasks(100)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|task| task.id)
                .collect()
        } else {
            Vec::new()
        };

        crate::domain::health_gate::system_health_assessment_with_contract_and_overdue(
            healthy_dogs,
            total_dogs,
            storage_ok,
            probes_degraded,
            &stale_tasks,
            &overdue_task_ids,
            Some(&contract_delta),
        )
    }
}

// ── PER-IP RATE LIMITER ──────────────────────────────────────

/// Per-IP token bucket rate limiter. Each IP gets its own bucket.
/// Uses tokio::sync::Mutex — never blocks a tokio worker thread.
/// Stale entries are evicted after 2 minutes of inactivity.
pub struct PerIpRateLimiter {
    buckets: Mutex<HashMap<IpAddr, TokenBucket>>,
    max_per_minute: u64,
}

impl std::fmt::Debug for PerIpRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerIpRateLimiter")
            .field("max_per_minute", &self.max_per_minute)
            .finish_non_exhaustive()
    }
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: std::time::Instant,
}

impl PerIpRateLimiter {
    pub fn new(max_per_minute: u64) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    /// Evict stale entries — called by background timer, NOT on the hot path.
    /// Previous design ran retain() on every request = O(N) under DDoS.
    pub async fn evict_stale(&self) {
        let mut buckets = self.buckets.lock().await;
        let now = std::time::Instant::now();
        buckets.retain(|_, b| now.duration_since(b.last_refill).as_secs() < 120);
    }

    /// Returns true if request from this IP is allowed.
    pub async fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = std::time::Instant::now();

        let max = self.max_per_minute as f64;
        let bucket = buckets.entry(ip).or_insert_with(|| TokenBucket {
            tokens: max,
            max_tokens: max,
            refill_rate: max / 60.0,
            last_refill: now,
        });

        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * bucket.refill_rate).min(bucket.max_tokens);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ── REQUEST / RESPONSE TYPES ───────────────────────────────

// ── POIH (Proof of Intelligence on-chain) ─────────────────

/// POST /permits/mint — request a behavioral wallet evaluation for PoIH attestation.
#[derive(Debug, Deserialize)]
pub struct MintPermitRequest {
    /// Solana wallet address to evaluate (base58, 32-44 chars).
    pub wallet_address: String,
    /// Proof source the caller is declaring (behavioral history used for attestation).
    pub proof_source: crate::domain::poih::ProofSource,
}

/// POST /permits/mint response.
#[derive(Debug, Serialize)]
pub struct MintPermitResponse {
    /// Permit details — present only when approved (WAG or HOWL).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permit: Option<crate::domain::poih::MintPermit>,
    /// Verdict kind: "wag", "howl", "growl", "bark", or "epoche".
    pub verdict_kind: String,
    /// Q-score from behavioral_dog (0.0–0.618).
    pub q_score: f64,
    /// Human-readable rejection reason — present only when not approved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
}

// ── PHONE-NUMBERS (CallShield) ─────────────────────────────

/// One entry in the blocklist: phone number + worst-first ordering data.
#[derive(Debug, Serialize)]
pub struct BlocklistEntry {
    /// E.164 format phone number as submitted to /judge.
    pub number: String,
    /// Sovereignty score (0.0 = worst spam, 1.0 = clean).
    pub sovereignty: f64,
    /// Composite Q-score across all six axioms.
    pub q_score: f64,
    /// Verdict kind: Bark | Growl | Wag | Howl.
    pub verdict: String,
}

/// GET /phone-numbers/blocklist response.
#[derive(Debug, Serialize)]
pub struct BlocklistResponse {
    pub numbers: Vec<BlocklistEntry>,
    pub count: usize,
    pub generated_at: String,
}

/// GET /phone-numbers/reporter-stats response (MVP placeholder).
#[derive(Debug, Serialize)]
pub struct ReporterStatsResponse {
    /// Agreement rate with kernel verdicts — None until per-device tracking is wired.
    pub agreement_rate: Option<f64>,
    /// Total reports submitted by this device. Always 0 in MVP (no per-device tracking yet).
    pub reports_total: u64,
    /// Device tier: "anonymous" | "trusted" | "verified".
    pub tier: String,
}

/// POST /dogs/register — register a new Dog at runtime.
#[derive(Debug, Deserialize)]
pub struct RegisterDogRequest {
    /// Unique Dog ID (must not collide with existing).
    pub name: String,
    /// OpenAI-compatible base URL (e.g. "http://host:8080/v1").
    pub base_url: String,
    /// Model name for prompt routing.
    pub model: String,
    /// Optional API key for authenticated backends.
    pub api_key: Option<String>,
    /// Max context tokens (0 = unknown/unlimited). Default: 4096.
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    /// Timeout in seconds. Default: 60.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_context_size() -> u32 {
    4096
}
fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Serialize)]
pub struct RegisterDogResponse {
    pub dog_id: String,
    pub calibration: String,
    pub roster_size: usize,
}

/// Tracks TTL for Dogs registered via POST /dogs/register.
/// Config-based Dogs (backends.toml) are permanent and not tracked here.
#[derive(Debug)]
pub struct RegisteredDog {
    pub registered_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
    pub ttl_secs: u64,
}

/// OS-level service entry — organism process table row.
/// Every running service registers here on startup and heartbeats to stay visible.
/// TTL: 120s default. Missing heartbeat = ServiceDown event + eviction.
#[derive(Debug, Clone)]
pub struct RegisteredService {
    /// Unique service identifier — matches systemd unit name (without .service).
    pub service_id: String,
    /// Infrastructure tier: 0=kernel/storage, 1=nervous system, 2=organs, 3=secondary, 4=ui.
    pub tier: u8,
    /// Node where this service runs (matches registry.json node name).
    pub node: String,
    /// Declared capabilities — what this service can do (observe, claim, wait_for_tasks, etc.).
    pub capabilities: Vec<String>,
    /// Git SHA or semver at registration time — for drift detection.
    pub version: Option<String>,
    pub registered_at: std::time::Instant,
    pub last_heartbeat: std::time::Instant,
    /// TTL seconds — service must heartbeat within this window or it's evicted.
    pub ttl_secs: u64,
}

// ── SERVICE API TYPES ─────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct RegisterServiceRequest {
    /// Unique service ID (e.g. "hermes-agent-executor", "organ-gasdf").
    pub service_id: String,
    /// Infrastructure tier 0-4.
    pub tier: u8,
    /// Node name from registry.json (e.g. "cynic-host-cortex", "cynic-vm-anvil").
    pub node: String,
    /// Declared capabilities (e.g. ["observe", "claim", "wait_for_tasks"]).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Optional version/git-sha for drift detection.
    pub version: Option<String>,
    /// Requested TTL in seconds (default: 120).
    pub ttl_secs: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct RegisterServiceResponse {
    pub service_id: String,
    pub status: String,
    pub ttl_secs: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct ServiceHeartbeatResponse {
    pub service_id: String,
    pub status: String,
    pub ttl_remaining_secs: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct ServiceUnregisterResponse {
    pub service_id: String,
    pub status: String,
}

/// Snapshot of a registered service — returned by GET /services.
#[derive(Debug, serde::Serialize)]
pub struct ServiceSnapshot {
    pub service_id: String,
    pub tier: u8,
    pub node: String,
    pub capabilities: Vec<String>,
    pub version: Option<String>,
    pub uptime_secs: u64,
    pub last_heartbeat_secs_ago: u64,
    pub ttl_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    pub dog_id: String,
    pub status: String,
    pub ttl_remaining_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct DeregisterResponse {
    pub dog_id: String,
    pub status: String,
    pub roster_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    /// Optional: evaluate with only these Dogs (by ID). If omitted, all Dogs are used.
    #[serde(default, deserialize_with = "deserialize_optional_dog_id_vec")]
    pub dogs: Option<Vec<String>>,
    /// Optional: disable crystal injection for A/B testing. Default: true.
    /// Accepts: true, false, or null/absent (→ true).
    #[serde(
        default = "default_true",
        deserialize_with = "deserialize_bool_or_null"
    )]
    pub crystals: bool,
    /// Optional: sensitivity level. "high" forces routing to sovereign (local) Dogs only.
    /// Use for private content: DMs, wallet seeds, API keys. Default: none (auto-detected).
    pub sensitivity: Option<String>,
    /// Optional: slot acquisition priority. Controls queuing behavior on sovereign inference.
    /// Values: "user" (30s wait, default), "hermes" (15s), "nightshift" (skip if busy), "background" (skip).
    pub priority: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Deserialize a bool that also accepts null (→ true).
/// Hermes Agent MCP sends crystals: null when the field is unset.
fn deserialize_bool_or_null<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Option<bool> = Option::deserialize(deserializer)?;
    Ok(v.unwrap_or(true))
}

/// Deserialize a backend label string, mapping the legacy label to the canonical backend id.
pub fn deserialize_dog_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == LEGACY_DOG_ID {
        Ok(CANONICAL_DOG_ID.to_string())
    } else {
        Ok(s)
    }
}

/// Deserialize an optional backend label string, mapping the legacy label to the canonical backend id.
pub fn deserialize_optional_dog_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.map(|s| {
        if s == LEGACY_DOG_ID {
            CANONICAL_DOG_ID.to_string()
        } else {
            s
        }
    }))
}

/// Deserialize an optional list of backend labels, mapping any legacy label to the canonical backend id.
pub fn deserialize_optional_dog_id_vec<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|vec| {
        vec.into_iter()
            .map(|s| {
                if s == LEGACY_DOG_ID {
                    CANONICAL_DOG_ID.to_string()
                } else {
                    s
                }
            })
            .collect()
    }))
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub domain: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
    pub timestamp: i64,
    pub dog_scores: Vec<DogScoreResponse>,
    /// Number of Dogs that contributed to this verdict (T9: transparency).
    /// Consumers can distinguish single-Dog from consensus.
    pub voter_count: usize,
    pub anomaly_detected: bool,
    pub max_disagreement: f64,
    pub anomaly_axiom: Option<String>,
    /// BLAKE3 integrity hash of this verdict
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity_hash: Option<String>,
    /// BLAKE3 hash of the previous verdict (chain link)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
    /// True if this verdict came from semantic cache (0 API calls consumed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<f64>,
    /// Enriched token data — present when domain=token-analysis and address was enriched via Helius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_data: Option<crate::domain::enrichment::TokenData>,
    /// The actual stimulus content sent to Dogs (may differ from request content if enriched).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stimulus_content: Option<String>,
    /// Dogs that failed during this evaluation (telemetry — why N Dogs voted, not 5).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_dogs: Vec<String>,
    /// Error reason per failed Dog (dog_id → error string).
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub failed_dog_errors: std::collections::BTreeMap<String, String>,
    /// Structured error kind per failed Dog (dog_id → error code).
    /// Codes: api_error | parse_error | rate_limited | timeout | slot_unavailable
    ///        network_unreachable | circuit_open | quality_gate_excluded
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub failed_dog_error_kinds: std::collections::BTreeMap<String, String>,
    /// Dogs excluded before evaluation by circuit breaker or quality gate (not attempted).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excluded_dogs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DogScoreResponse {
    pub dog_id: String,
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    /// Raw scores before phi_bound — what the model actually produced.
    pub raw_fidelity: f64,
    pub raw_phi: f64,
    pub raw_verify: f64,
    pub raw_culture: f64,
    pub raw_burn: f64,
    pub raw_sovereignty: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct DogHealthResponse {
    pub id: String,
    pub kind: String,
    pub sovereign: bool,
    pub circuit: String,
    pub failures: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_since_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_allows_within_limit() {
        let limiter = PerIpRateLimiter::new(3);
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(limiter.check(ip).await);
        assert!(!limiter.check(ip).await); // 4th request rejected
        // Different IP gets its own bucket
        let ip2: std::net::IpAddr = "10.0.0.1".parse().unwrap();
        assert!(ip2 != ip);
        assert!(limiter.check(ip2).await); // separate bucket, allowed
    }

    #[test]
    fn judge_request_crystals_null_defaults_to_true() {
        let json = r#"{"content":"test","crystals":null}"#;
        let req: JudgeRequest = serde_json::from_str(json).unwrap();
        assert!(req.crystals, "crystals: null should default to true");
    }

    #[test]
    fn judge_request_crystals_absent_defaults_to_true() {
        let json = r#"{"content":"test"}"#;
        let req: JudgeRequest = serde_json::from_str(json).unwrap();
        assert!(req.crystals, "crystals absent should default to true");
    }

    #[test]
    fn judge_request_crystals_false_works() {
        let json = r#"{"content":"test","crystals":false}"#;
        let req: JudgeRequest = serde_json::from_str(json).unwrap();
        assert!(!req.crystals);
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct PublishMeshKeyRequest {
    pub mesh_ip: String,
    pub public_key: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PublishMeshKeyResponse {
    pub status: String,
    pub mesh_ip: String,
    pub public_key: String,
}
