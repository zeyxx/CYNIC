//! Local Unix-peer agent session bootstrap shared by MCP and CLI projections.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use tokio_util::sync::CancellationToken;

use cynic_protocol::{
    AGENT_ROOT_LAUNCH_REQUEST_SCHEMA, AGENT_SESSION_RENEWAL_REQUEST_SCHEMA, AgentBudget,
    AgentCapabilityGrant, AgentRootLaunchRequest, AgentSessionCredentialKind,
    AgentSessionRenewalRequest, AgentSessionTransport,
};

const KERNEL_ORIGIN: &str = "http://cynic.local";
const SESSION_HEADER: &str = "X-Cynic-Agent-Session";
const RENEWAL_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct LocalAgentIdentity {
    pub agent_id: String,
    pub agent_type: String,
}

impl LocalAgentIdentity {
    pub fn resolve(default_type: &str) -> Result<Self, String> {
        let agent_type = component(
            std::env::var("CYNIC_AGENT_TYPE")
                .ok()
                .as_deref()
                .unwrap_or(default_type),
            default_type,
        );
        let agent_id = std::env::var("CYNIC_AGENT_ID").unwrap_or_else(|_| {
            format!(
                "{agent_type}:{}:{}",
                component(&hostname(), "node"),
                std::process::id()
            )
        });
        if agent_id.is_empty()
            || agent_id.chars().count() > 64
            || agent_id.chars().any(char::is_control)
        {
            return Err("CYNIC_AGENT_ID must be 1-64 non-control characters".to_string());
        }
        Ok(Self {
            agent_id,
            agent_type,
        })
    }
}

#[derive(Clone)]
pub struct LocalAgentSessionClient {
    socket_path: PathBuf,
    pub node_id: String,
    pub agent_id: String,
    requested_ttl_ms: u64,
    state: Arc<RwLock<LocalAgentSessionState>>,
}

struct LocalAgentSessionState {
    client: reqwest::Client,
    session_id: String,
    expires_at_unix_ms: u64,
    generation: u64,
}

impl std::fmt::Debug for LocalAgentSessionClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalAgentSessionClient")
            .field("socket_path", &self.socket_path)
            .field("node_id", &self.node_id)
            .field("agent_id", &self.agent_id)
            .field("requested_ttl_ms", &self.requested_ttl_ms)
            .field("session_active", &self.is_active())
            .finish()
    }
}

impl LocalAgentSessionClient {
    pub fn disconnected(
        socket_path: impl Into<PathBuf>,
        agent_id: impl Into<String>,
    ) -> Result<Self, String> {
        let socket_path = socket_path.into();
        let client = client_for_socket(&socket_path, HeaderMap::new())?;
        Ok(Self {
            socket_path,
            node_id: "unknown".to_string(),
            agent_id: agent_id.into(),
            requested_ttl_ms: 0,
            state: Arc::new(RwLock::new(LocalAgentSessionState {
                client,
                session_id: String::new(),
                expires_at_unix_ms: 0,
                generation: 0,
            })),
        })
    }

    pub fn mock_active(
        socket_path: impl Into<PathBuf>,
        agent_id: impl Into<String>,
    ) -> Result<Self, String> {
        let socket_path = socket_path.into();
        let client = client_for_socket(&socket_path, HeaderMap::new())?;
        Ok(Self {
            socket_path,
            node_id: "test-node".to_string(),
            agent_id: agent_id.into(),
            requested_ttl_ms: 3600000,
            state: Arc::new(RwLock::new(LocalAgentSessionState {
                client,
                session_id: "mock_session_123".to_string(),
                expires_at_unix_ms: u64::MAX,
                generation: 1,
            })),
        })
    }

    pub async fn bootstrap(
        socket_path: impl Into<PathBuf>,
        identity: &LocalAgentIdentity,
    ) -> Result<Self, String> {
        let socket_path = socket_path.into();
        let bootstrap = client_for_socket(&socket_path, HeaderMap::new())?;
        let identity_response = bootstrap
            .get(format!("{KERNEL_ORIGIN}/identity"))
            .send()
            .await
            .map_err(|_| "kernel_local_identity_unreachable".to_string())?;
        if !identity_response.status().is_success() {
            return Err("kernel_local_identity_rejected".to_string());
        }
        let kernel: KernelIdentity = identity_response
            .json()
            .await
            .map_err(|_| "kernel_local_identity_invalid".to_string())?;
        let requested_at_unix_ms = now_unix_ms();
        let requested_ttl_ms = requested_ttl_ms()?;
        let launch = AgentRootLaunchRequest {
            schema: AGENT_ROOT_LAUNCH_REQUEST_SCHEMA.to_string(),
            request_id: format!(
                "local-session-{}-{requested_at_unix_ms}",
                std::process::id()
            ),
            agent_id: identity.agent_id.clone(),
            agent_type: identity.agent_type.clone(),
            node_id: kernel.node_id.clone(),
            objective: std::env::var("CYNIC_AGENT_OBJECTIVE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "bounded local CYNIC projection session".to_string()),
            requested_at_unix_ms,
            requested_ttl_ms,
            transport: AgentSessionTransport::LocalUnixPeer,
            credential_kind: AgentSessionCredentialKind::LocalPeerBinding,
            capabilities: requested_capabilities()?,
            budget: AgentBudget {
                context_tokens: 100_000,
                actions: 100,
                children: 0,
                wall_time_ms: requested_ttl_ms,
            },
            expected_proofs: vec![
                "agent_session_read_back".to_string(),
                "coordination_identity_read_back".to_string(),
            ],
            confirmation_key_sha256: None,
        };
        let response = bootstrap
            .post(format!("{KERNEL_ORIGIN}/auth/agent/root-launch"))
            .json(&launch)
            .send()
            .await
            .map_err(|_| "kernel_local_session_launch_unreachable".to_string())?;
        if !response.status().is_success() {
            return Err("kernel_local_session_launch_rejected".to_string());
        }
        let issued: RootLaunchResponse = response
            .json()
            .await
            .map_err(|_| "kernel_local_session_launch_read_back_invalid".to_string())?;
        let session = issued
            .session
            .filter(|session| {
                issued.status == "issued"
                    && session.agent_id == identity.agent_id
                    && session.node_id == kernel.node_id
                    && !session.secret
                    && session.generation == 1
                    && session.expires_at_unix_ms > requested_at_unix_ms
            })
            .ok_or_else(|| "kernel_local_session_launch_read_back_mismatch".to_string())?;
        let mut headers = HeaderMap::new();
        headers.insert(
            SESSION_HEADER,
            HeaderValue::from_str(&session.session_id)
                .map_err(|_| "kernel_local_session_header_invalid".to_string())?,
        );
        let session_client = client_for_socket(&socket_path, headers)?;
        Ok(Self {
            socket_path,
            node_id: kernel.node_id,
            agent_id: identity.agent_id.clone(),
            requested_ttl_ms,
            state: Arc::new(RwLock::new(LocalAgentSessionState {
                client: session_client,
                session_id: session.session_id,
                expires_at_unix_ms: session.expires_at_unix_ms,
                generation: session.generation,
            })),
        })
    }

    pub fn client(&self) -> reqwest::Client {
        self.state
            .read()
            .expect("local agent session state lock poisoned")
            .client
            .clone()
    }

    pub fn base_url(&self) -> &'static str {
        KERNEL_ORIGIN
    }

    pub fn url(&self, path: &str) -> String {
        format!("{KERNEL_ORIGIN}{path}")
    }

    pub fn is_active(&self) -> bool {
        self.state.read().is_ok_and(|state| {
            !state.session_id.is_empty() && state.expires_at_unix_ms > now_unix_ms()
        })
    }

    pub fn mark_unavailable(&self) {
        if let Ok(mut state) = self.state.write() {
            state.session_id.clear();
            state.expires_at_unix_ms = 0;
        }
    }

    pub async fn revoke(&self) -> Result<(), String> {
        let response = self
            .client()
            .post(self.url("/auth/agent/session/revoke"))
            .send()
            .await
            .map_err(|_| "kernel_local_session_revocation_unreachable".to_string())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err("kernel_local_session_revocation_rejected".to_string())
        }
    }

    pub async fn maintain(&self, shutdown: CancellationToken) -> Result<(), String> {
        loop {
            let delay = match self.renewal_delay() {
                Ok(delay) => delay,
                Err(_) => return Ok(()),
            };
            tokio::select! {
                () = tokio::time::sleep(delay) => {
                    if self.renew().await.is_err() {
                        tokio::select! {
                            () = tokio::time::sleep(RENEWAL_RETRY_DELAY) => {},
                            () = shutdown.cancelled() => return Ok(()),
                        }
                    }
                },
                () = shutdown.cancelled() => return Ok(()),
            }
        }
    }

    async fn renew(&self) -> Result<(), String> {
        let (client, previous_session_id, previous_generation) = self
            .state
            .read()
            .map(|state| {
                (
                    state.client.clone(),
                    state.session_id.clone(),
                    state.generation,
                )
            })
            .map_err(|_| "local agent session state lock poisoned".to_string())?;
        let requested_at_unix_ms = now_unix_ms();
        let request = AgentSessionRenewalRequest {
            schema: AGENT_SESSION_RENEWAL_REQUEST_SCHEMA.to_string(),
            request_id: format!("local-renew-{}-{requested_at_unix_ms}", std::process::id()),
            requested_at_unix_ms,
            requested_ttl_ms: self.requested_ttl_ms,
        };
        let response = client
            .post(self.url("/auth/agent/session/renew"))
            .json(&request)
            .send()
            .await
            .map_err(|_| "kernel_local_session_renewal_unreachable".to_string())?;
        if !response.status().is_success() {
            return Err("kernel_local_session_renewal_rejected".to_string());
        }
        let proof: RenewalResponse = response
            .json()
            .await
            .map_err(|_| "kernel_local_session_renewal_read_back_invalid".to_string())?;
        let session = proof
            .session
            .filter(|session| {
                proof.status == "rotated"
                    && session.session_id != previous_session_id
                    && session.agent_id == self.agent_id
                    && session.node_id == self.node_id
                    && previous_generation.checked_add(1) == Some(session.generation)
                    && !session.secret
                    && session.expires_at_unix_ms > requested_at_unix_ms
            })
            .ok_or_else(|| "kernel_local_session_renewal_read_back_mismatch".to_string())?;
        let mut headers = HeaderMap::new();
        headers.insert(
            SESSION_HEADER,
            HeaderValue::from_str(&session.session_id)
                .map_err(|_| "kernel_local_session_header_invalid".to_string())?,
        );
        let replacement_client = client_for_socket(&self.socket_path, headers)?;
        let mut state = self
            .state
            .write()
            .map_err(|_| "local agent session state lock poisoned".to_string())?;
        if state.session_id != previous_session_id {
            return Err("kernel_local_session_changed_during_renewal".to_string());
        }
        state.client = replacement_client;
        state.session_id = session.session_id;
        state.expires_at_unix_ms = session.expires_at_unix_ms;
        state.generation = session.generation;
        Ok(())
    }

    fn renewal_delay(&self) -> Result<Duration, String> {
        let expires_at_unix_ms = self
            .state
            .read()
            .map(|state| state.expires_at_unix_ms)
            .map_err(|_| "local agent session state lock poisoned".to_string())?;
        let remaining = expires_at_unix_ms.saturating_sub(now_unix_ms());
        if remaining == 0 {
            return Err("kernel_local_session_expired".to_string());
        }
        let safety_margin = (self.requested_ttl_ms / 3).max(10_000);
        Ok(Duration::from_millis(
            remaining.saturating_sub(safety_margin).max(1_000),
        ))
    }
}

#[async_trait::async_trait]
impl crate::ports::build_workload_submission_port::BuildWorkloadSubmissionPort
    for LocalAgentSessionClient
{
    async fn submit(
        &self,
        delivery: &cynic_protocol::AgentWorkloadDelivery,
    ) -> Result<crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack, String>
    {
        let response = self
            .client()
            .post(self.url("/agent-workloads"))
            .json(delivery)
            .send()
            .await
            .map_err(|_| "kernel_workload_submission_unreachable".to_string())?;
        let http_status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|_| "kernel_workload_submission_read_back_unreachable".to_string())?;
        Ok(
            crate::ports::build_workload_submission_port::BuildWorkloadSubmissionReadBack {
                http_status,
                body: serde_json::from_str(&body)
                    .unwrap_or_else(|_| serde_json::json!({"raw": body})),
            },
        )
    }
}

#[async_trait::async_trait]
impl crate::ports::build_workload_observation_port::BuildWorkloadObservationPort
    for LocalAgentSessionClient
{
    async fn observe_workload(
        &self,
        workload_id: &str,
    ) -> Result<
        crate::ports::build_workload_observation_port::BuildWorkloadObservationReadBack,
        String,
    > {
        if workload_id.is_empty() || workload_id.chars().any(char::is_control) {
            return Err("kernel_workload_observation_id_invalid".to_string());
        }
        let response = self
            .client()
            .get(self.url(&format!("/agent-workloads/{workload_id}")))
            .send()
            .await
            .map_err(|_| "kernel_workload_observation_unreachable".to_string())?;
        let http_status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|_| "kernel_workload_observation_read_back_unreachable".to_string())?;
        Ok(
            crate::ports::build_workload_observation_port::BuildWorkloadObservationReadBack {
                http_status,
                body: serde_json::from_str(&body)
                    .unwrap_or_else(|_| serde_json::json!({"raw": body})),
            },
        )
    }
}

#[async_trait::async_trait]
impl crate::ports::runtime_release_declaration_port::RuntimeReleaseDeclarationPort
    for LocalAgentSessionClient
{
    async fn declare_runtime_release(
        &self,
        declaration: &cynic_protocol::RuntimeReleaseDeclaration,
    ) -> Result<
        crate::ports::runtime_release_declaration_port::RuntimeReleaseDeclarationReadBack,
        String,
    > {
        let target_node_id = &declaration.desired_state.node_id;
        if target_node_id.is_empty() || target_node_id.chars().any(char::is_control) {
            return Err("kernel_runtime_release_target_invalid".to_string());
        }
        let response = self
            .client()
            .put(self.url(&format!("/node-control/{target_node_id}/desired-state")))
            .json(declaration)
            .send()
            .await
            .map_err(|_| "kernel_runtime_release_unreachable".to_string())?;
        let http_status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|_| "kernel_runtime_release_read_back_unreachable".to_string())?;
        Ok(
            crate::ports::runtime_release_declaration_port::RuntimeReleaseDeclarationReadBack {
                http_status,
                body: serde_json::from_str(&body)
                    .unwrap_or_else(|_| serde_json::json!({"raw": body})),
            },
        )
    }
}

pub fn declared_kernel_socket() -> PathBuf {
    std::env::var_os("CYNIC_KERNEL_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            cynic_host::runtime_authority::default_runtime_authority_paths().kernel_socket_path
        })
}

fn client_for_socket(path: &Path, headers: HeaderMap) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .unix_socket(path)
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| "kernel_local_client_initialization_failed".to_string())
}

fn requested_capabilities() -> Result<Vec<AgentCapabilityGrant>, String> {
    match std::env::var("CYNIC_AGENT_CAPABILITIES_JSON") {
        Ok(raw) if !raw.trim().is_empty() => serde_json::from_str(&raw)
            .map_err(|_| "CYNIC_AGENT_CAPABILITIES_JSON invalid".to_string()),
        _ => Ok(cynic_host::agent_access::default_local_agent_capabilities()),
    }
}

fn requested_ttl_ms() -> Result<u64, String> {
    let value = std::env::var("CYNIC_AGENT_SESSION_TTL_MS")
        .unwrap_or_else(|_| "900000".to_string())
        .parse::<u64>()
        .map_err(|_| "CYNIC_AGENT_SESSION_TTL_MS must be an integer".to_string())?;
    if !(10_000..=3_600_000).contains(&value) {
        return Err("CYNIC_AGENT_SESSION_TTL_MS is outside the allowed range".to_string());
    }
    Ok(value)
}

fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "node".to_string())
}
fn component(value: &str, fallback: &str) -> String {
    let value: String = value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .take(12)
        .collect();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}
fn now_unix_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

#[derive(Deserialize)]
struct KernelIdentity {
    node_id: String,
}
#[derive(Deserialize)]
struct RootLaunchResponse {
    status: String,
    session: Option<IssuedSession>,
}
#[derive(Deserialize)]
struct RenewalResponse {
    status: String,
    session: Option<IssuedSession>,
}
#[derive(Deserialize)]
struct IssuedSession {
    session_id: String,
    agent_id: String,
    node_id: String,
    expires_at_unix_ms: u64,
    generation: u64,
    secret: bool,
}
