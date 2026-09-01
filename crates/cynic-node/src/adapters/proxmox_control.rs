use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::adapters::ed25519_credential_lease_verifier::Ed25519CredentialLeaseVerifier;
use crate::ports::credential_lease_verifier_port::CredentialLeaseVerifierPort;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProxmoxControlStatus {
    pub schema: &'static str,
    pub status: &'static str,
    pub configured: ProxmoxControlConfiguration,
    pub api_reachable: Option<bool>,
    pub resource_count: Option<usize>,
    pub blockers: Vec<&'static str>,
    pub next_action: Option<&'static str>,
    pub secret_exposed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProxmoxControlConfiguration {
    pub endpoint: bool,
    pub client_identity: bool,
    pub credential: bool,
    pub custom_ca: bool,
    /// Scope and provenance are part of the observation contract.  Presence
    /// alone must never be mistaken for provider enrollment.
    pub identity_scope: &'static str,
    pub endpoint_source: Option<&'static str>,
    pub client_identity_source: Option<&'static str>,
    pub credential_source: Option<&'static str>,
}

#[derive(Debug, Default, Deserialize)]
struct ProxmoxControlFile {
    endpoint: Option<String>,
    token_id: Option<String>,
    token_secret_file: Option<String>,
    ca_file: Option<String>,
    credential_lease_metadata_file: Option<String>,
    credential_authority_public_key_file: Option<String>,
}

struct ResolvedProxmoxControl {
    endpoint: Option<String>,
    token_id: Option<String>,
    token_secret_file: PathBuf,
    ca_file: Option<String>,
    lease_metadata_ref: Option<String>,
    authority_public_key_ref: Option<String>,
}

/// Proxmox API adapter owned by the node/provider layer.
///
/// Callers provide already-admitted argv. Projection layers never receive the
/// provider credential and never construct provider HTTP requests directly.
pub struct ProxmoxControlAdapter {
    base_url: String,
    client_id: String,
    credential: String,
    client: reqwest::Client,
    /// Path to CA PEM file used to build the TLS client, retained so callers
    /// (e.g. the blocking REST observer in cynic-cli) can build a compatible
    /// client without re-resolving the configuration.
    pub ca_file: Option<String>,
}

impl std::fmt::Debug for ProxmoxControlAdapter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxmoxControlAdapter")
            .field("base_url", &self.base_url)
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}

impl ProxmoxControlAdapter {
    /// Observe only the process-visible provider configuration.  This is
    /// deliberately shared with identity projections so they cannot invent a
    /// second enrollment interpretation; it performs no network I/O and never
    /// returns credential material.
    pub fn observe_process_configuration() -> ProxmoxControlConfiguration {
        observe_configuration()
    }

    /// Observe provider-control readiness without returning credentials or
    /// machine-local secret paths. This is the read-back surface used by CLI
    /// and MCP projections before they propose any provider mutation.
    pub async fn observe_from_env(timeout: Duration) -> ProxmoxControlStatus {
        let configured = observe_configuration();
        let blockers = configuration_blockers(&configured);
        if !blockers.is_empty() {
            return ProxmoxControlStatus {
                schema: "cynic.node.proxmox_control_status.v1",
                status: "enrollment_required",
                configured,
                api_reachable: None,
                resource_count: None,
                blockers,
                next_action: Some(
                    "enroll provider endpoint, token identity, and token secret through the host secret-injection boundary",
                ),
                secret_exposed: false,
            };
        }

        let adapter = match Self::from_env(timeout) {
            Ok(adapter) => adapter,
            Err(_) => {
                return ProxmoxControlStatus {
                    schema: "cynic.node.proxmox_control_status.v1",
                    status: "configuration_invalid",
                    configured,
                    api_reachable: None,
                    resource_count: None,
                    blockers: vec!["provider_configuration_invalid"],
                    next_action: Some("repair provider configuration and run status read-back"),
                    secret_exposed: false,
                };
            }
        };
        match adapter.cluster_resources().await {
            Ok(payload) => ProxmoxControlStatus {
                schema: "cynic.node.proxmox_control_status.v1",
                status: "ready",
                configured,
                api_reachable: Some(true),
                resource_count: payload["resources"].as_array().map(Vec::len),
                blockers: Vec::new(),
                next_action: None,
                secret_exposed: false,
            },
            Err(_) => ProxmoxControlStatus {
                schema: "cynic.node.proxmox_control_status.v1",
                status: "provider_unreachable_or_rejected",
                configured,
                api_reachable: Some(false),
                resource_count: None,
                blockers: vec!["provider_api_read_back_failed"],
                next_action: Some(
                    "verify provider reachability, CA trust, token validity, and read permission",
                ),
                secret_exposed: false,
            },
        }
    }

    pub fn from_env(timeout: Duration) -> Result<Self, String> {
        let resolved = resolve_control_configuration();
        let base_url = resolved
            .endpoint
            .ok_or_else(|| "Proxmox endpoint is not configured".to_string())?;
        let client_id = resolved
            .token_id
            .ok_or_else(|| "Proxmox token identity is not configured".to_string())?;
        let desired = cynic_protocol::provider::ProviderControlDesiredState {
            schema: cynic_protocol::provider::PROVIDER_CONTROL_DESIRED_STATE_SCHEMA.to_string(),
            provider: "proxmox".to_string(),
            endpoint: base_url.clone(),
            token_identity: client_id.clone(),
            credential_ref: resolved.token_secret_file.display().to_string(),
            ca_ref: resolved.ca_file.clone(),
            credential_lease_metadata_ref: resolved.lease_metadata_ref.clone(),
            credential_authority_public_key_ref: resolved.authority_public_key_ref.clone(),
        };
        let blockers = cynic_protocol::provider::validate_provider_control_desired_state(&desired);
        if !blockers.is_empty() {
            return Err(format!(
                "Proxmox control desired state is invalid: {}",
                blockers.join(",")
            ));
        }
        let credential = resolve_credential(&resolved.token_secret_file)?;
        let ca_file = resolved.ca_file;
        let mut client = reqwest::Client::builder().timeout(timeout.max(Duration::from_secs(5)));
        if let Some(ca_path) = &ca_file {
            let pem = std::fs::read(ca_path)
                .map_err(|error| format!("cannot read Proxmox CA file {ca_path}: {error}"))?;
            let certificate = reqwest::Certificate::from_pem(&pem)
                .map_err(|error| format!("invalid Proxmox CA file {ca_path}: {error}"))?;
            client = client.add_root_certificate(certificate);
        }
        let client = client
            .build()
            .map_err(|error| format!("cannot build Proxmox API client: {error}"))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id,
            credential,
            client,
            ca_file,
        })
    }

    /// Return the resolved connection triple (endpoint, token_id, token_secret)
    /// for callers that need to build a blocking REST client from the same
    /// resolved configuration. The credential is already in memory; this does
    /// not re-read secrets.
    pub fn connection_parts(&self) -> (&str, &str, &str) {
        (&self.base_url, &self.client_id, &self.credential)
    }

    /// Observe the provider credential as a lease without returning its
    /// material. Proxmox API tokens do not expose issuer, audience, expiry, or
    /// rotation generation through this local file adapter, so those fields
    /// remain explicit unknowns until a rotation authority is configured.
    pub fn observe_credential_lease(
        desired: &cynic_protocol::CredentialLeaseDesiredState,
    ) -> cynic_protocol::CredentialLeaseObservation {
        let resolved = resolve_control_configuration();
        let path = std::path::Path::new(&desired.secret_ref);
        let metadata = std::fs::symlink_metadata(path).ok();
        let regular = metadata
            .as_ref()
            .is_some_and(|value| value.file_type().is_file() && !value.file_type().is_symlink());
        let material_digest = regular
            .then(|| {
                std::fs::read(path)
                    .ok()
                    .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
            })
            .flatten();
        let mut unknowns = vec![
            "credential_lease_subject_not_bound_by_proxmox_file_adapter".to_string(),
            "credential_lease_audience_not_bound_by_proxmox_file_adapter".to_string(),
            "credential_lease_expiry_not_exposed_by_proxmox_api_token".to_string(),
            "credential_lease_generation_not_exposed_by_proxmox_file_adapter".to_string(),
        ];
        if desired.secret_ref != resolved.token_secret_file.display().to_string() {
            unknowns.push("credential_lease_secret_ref_not_current_runtime_binding".to_string());
        }
        let metadata = desired
            .lease_metadata_ref
            .as_deref()
            .and_then(|value| std::fs::read_to_string(value).ok())
            .and_then(|value| {
                serde_json::from_str::<cynic_protocol::CredentialLeaseMetadata>(&value).ok()
            });
        let metadata_blockers = metadata.as_ref().map_or_else(
            || vec!["credential_lease_metadata_unreadable".to_string()],
            |value| {
                cynic_protocol::validate_credential_lease_metadata(
                    value,
                    "proxmox",
                    material_digest.as_deref(),
                )
            },
        );
        if let Some(value) = metadata.as_ref().filter(|_| metadata_blockers.is_empty()) {
            unknowns.retain(|item| {
                !matches!(
                    item.as_str(),
                    "credential_lease_subject_not_bound_by_proxmox_file_adapter"
                        | "credential_lease_audience_not_bound_by_proxmox_file_adapter"
                        | "credential_lease_expiry_not_exposed_by_proxmox_api_token"
                        | "credential_lease_generation_not_exposed_by_proxmox_file_adapter"
                )
            });
            unknowns.push("credential_lease_metadata_signature_verification_not_wired".to_string());
            return cynic_protocol::CredentialLeaseObservation {
                schema: cynic_protocol::CREDENTIAL_LEASE_OBSERVATION_SCHEMA.to_string(),
                state: if regular {
                    cynic_protocol::CredentialLeaseState::Present
                } else {
                    cynic_protocol::CredentialLeaseState::Unknown
                },
                provider: Some(value.provider.clone()),
                subject: Some(value.subject.clone()),
                audience: Some(value.audience.clone()),
                generation: Some(value.generation),
                expires_at_unix_seconds: Some(value.expires_at_unix_seconds),
                material_digest,
                evidence_refs: vec!["credential-lease-metadata:digest-match".to_string()],
                unknowns,
            };
        }
        if desired.lease_metadata_ref.is_some() {
            unknowns.push(format!(
                "credential_lease_metadata_read_back_invalid:{}",
                metadata_blockers.join(",")
            ));
        } else {
            unknowns.push("credential_lease_metadata_ref_missing".to_string());
        }
        cynic_protocol::CredentialLeaseObservation {
            schema: cynic_protocol::CREDENTIAL_LEASE_OBSERVATION_SCHEMA.to_string(),
            state: if regular {
                cynic_protocol::CredentialLeaseState::Present
            } else if metadata.is_none() {
                cynic_protocol::CredentialLeaseState::Absent
            } else {
                cynic_protocol::CredentialLeaseState::Unknown
            },
            provider: Some("proxmox".to_string()),
            subject: None,
            audience: None,
            generation: None,
            expires_at_unix_seconds: None,
            material_digest,
            evidence_refs: Vec::new(),
            unknowns,
        }
    }

    pub fn verify_credential_lease_metadata<V: CredentialLeaseVerifierPort>(
        desired: &cynic_protocol::CredentialLeaseDesiredState,
        verifier: &V,
    ) -> Result<Vec<String>, String> {
        let metadata_path = desired
            .lease_metadata_ref
            .as_deref()
            .ok_or_else(|| "credential_lease_metadata_ref_missing".to_string())?;
        let metadata: cynic_protocol::CredentialLeaseMetadata = serde_json::from_str(
            &std::fs::read_to_string(metadata_path)
                .map_err(|error| format!("credential_lease_metadata_read_failed:{error}"))?,
        )
        .map_err(|error| format!("credential_lease_metadata_decode_failed:{error}"))?;
        let digest = std::fs::read(&desired.secret_ref)
            .ok()
            .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))));
        verifier.verify(&metadata, &desired.provider, digest.as_deref())
    }

    pub fn observe_credential_lease_verified<V: CredentialLeaseVerifierPort>(
        desired: &cynic_protocol::CredentialLeaseDesiredState,
        verifier: &V,
    ) -> cynic_protocol::CredentialLeaseObservation {
        let mut observation = Self::observe_credential_lease(desired);
        match Self::verify_credential_lease_metadata(desired, verifier) {
            Ok(mut evidence_refs) => {
                observation.unknowns.retain(|unknown| {
                    unknown != "credential_lease_metadata_signature_verification_not_wired"
                });
                observation.evidence_refs.append(&mut evidence_refs);
                observation.evidence_refs.sort();
                observation.evidence_refs.dedup();
            }
            Err(error) => observation.unknowns.push(format!(
                "credential_lease_metadata_signature_verification_failed:{error}"
            )),
        }
        observation.unknowns.sort();
        observation.unknowns.dedup();
        observation
    }

    /// Observe the node's configured Proxmox lease proof without requiring a
    /// caller to reconstruct desired state or secret paths. Missing runtime
    /// proof inputs remain explicit unknowns.
    pub fn observe_runtime_credential_lease() -> cynic_protocol::CredentialLeaseObservation {
        let resolved = resolve_control_configuration();
        let desired = cynic_protocol::CredentialLeaseDesiredState {
            schema: cynic_protocol::CREDENTIAL_LEASE_DESIRED_STATE_SCHEMA.to_string(),
            provider: "proxmox".to_string(),
            secret_ref: resolved.token_secret_file.display().to_string(),
            lease_metadata_ref: resolved.lease_metadata_ref.clone(),
            subject: resolved.token_id.unwrap_or_default(),
            audience: optional_env("CYNIC_PROXMOX_CREDENTIAL_AUDIENCE").unwrap_or_default(),
            renewal_window_seconds: optional_env("CYNIC_PROXMOX_CREDENTIAL_RENEWAL_WINDOW_SECONDS")
                .and_then(|value| value.parse().ok())
                .unwrap_or(3_600),
        };
        let Some(authority_key_path) = resolved.authority_public_key_ref else {
            let mut observation = Self::observe_credential_lease(&desired);
            observation
                .unknowns
                .push("credential_lease_authority_public_key_not_configured".to_string());
            observation.unknowns.sort();
            observation.unknowns.dedup();
            return observation;
        };
        let verifier = Ed25519CredentialLeaseVerifier::new(authority_key_path);
        Self::observe_credential_lease_verified(&desired, &verifier)
    }

    pub async fn cluster_resources(&self) -> Result<Value, String> {
        let payload = self.cluster_resources_raw().await?;
        Ok(json!({
            "status": "ok",
            "resources": payload.get("data").cloned().unwrap_or_else(|| json!([])),
        }))
    }

    pub async fn cluster_resources_raw(&self) -> Result<Value, String> {
        self.get_json("/api2/json/cluster/resources?type=vm").await
    }

    pub async fn nodes(&self) -> Result<Value, String> {
        self.get_json("/api2/json/nodes").await
    }

    pub async fn node_resources(&self, node: &str, kind: &str) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        if !matches!(kind, "qemu" | "lxc") {
            return Err("unsupported Proxmox resource kind".to_string());
        }
        self.get_json(&format!("/api2/json/nodes/{node}/{kind}"))
            .await
    }

    pub async fn lxc_config(&self, node: &str, vmid: &str) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        validate_provider_component("vmid", vmid)?;
        self.get_json(&format!("/api2/json/nodes/{node}/lxc/{vmid}/config"))
            .await
    }

    pub async fn lxc_resize(
        &self,
        node: &str,
        vmid: &str,
        disk: &str,
        size_gib: u64,
    ) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        validate_provider_component("vmid", vmid)?;
        validate_provider_component("disk", disk)?;
        if size_gib == 0 {
            return Err("Proxmox LXC resize size must be non-zero".to_string());
        }
        self.post_form_json(
            &format!("/api2/json/nodes/{node}/lxc/{vmid}/resize"),
            &[("disk", disk.to_string()), ("size", format!("{size_gib}G"))],
        )
        .await
    }

    pub async fn task_status(&self, node: &str, upid: &str) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        if upid.is_empty()
            || upid.len() > 512
            || !upid.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_' | b'.')
            })
        {
            return Err("invalid Proxmox task id".to_string());
        }
        self.get_json(&format!("/api2/json/nodes/{node}/tasks/{upid}/status"))
            .await
    }

    pub async fn resource_action(
        &self,
        node: &str,
        resource_kind: &str,
        vmid: &str,
        action: &str,
    ) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        validate_provider_component("vmid", vmid)?;
        if !matches!(resource_kind, "qemu" | "lxc") {
            return Err("unsupported Proxmox resource kind".to_string());
        }
        if !matches!(action, "start" | "stop" | "reboot" | "suspend" | "resume") {
            return Err("unsupported Proxmox resource action".to_string());
        }
        self.post_form_json(
            &format!("/api2/json/nodes/{node}/{resource_kind}/{vmid}/status/{action}"),
            &[],
        )
        .await
    }

    pub async fn provision_qemu(
        &self,
        node: &str,
        vmid: u64,
        archive: String,
        name: String,
        storage: String,
    ) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        self.post_form_json(
            &format!("/api2/json/nodes/{node}/qemu"),
            &[
                ("vmid", vmid.to_string()),
                ("archive", archive),
                ("name", name),
                ("storage", storage),
            ],
        )
        .await
    }

    pub async fn qemu_guest_exec(
        &self,
        node: &str,
        vmid: &str,
        argv: &[String],
        timeout: Duration,
    ) -> Result<Value, String> {
        validate_provider_component("node", node)?;
        validate_provider_component("vmid", vmid)?;
        if argv.is_empty() || argv.iter().any(|argument| argument.contains('\0')) {
            return Err("guest argv must be non-empty and contain no NUL bytes".to_string());
        }
        let path = format!("/api2/json/nodes/{node}/qemu/{vmid}/agent/exec");
        let form = argv
            .iter()
            .map(|argument| ("command", argument.clone()))
            .collect::<Vec<_>>();
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth_header())
            .form(&form)
            .send()
            .await
            .map_err(|error| format!("Proxmox guest exec request failed: {error}"))?;
        let payload = response_json(response, &path).await?;
        let pid = payload["data"]["pid"]
            .as_i64()
            .ok_or_else(|| "Proxmox guest exec response omitted data.pid".to_string())?;
        let status_path =
            format!("/api2/json/nodes/{node}/qemu/{vmid}/agent/exec-status?pid={pid}");
        let started = Instant::now();
        while started.elapsed() < timeout.max(Duration::from_secs(1)) {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let status = self.get_json(&status_path).await?;
            if status["data"]["exited"].as_i64() == Some(1) {
                return Ok(json!({
                    "status": "completed",
                    "provider": "proxmox_qemu_guest_agent",
                    "node": node,
                    "vmid": vmid,
                    "pid": pid,
                    "argv": argv,
                    "exit_code": status["data"]["exitcode"].as_i64().unwrap_or(-1),
                    "stdout": status["data"]["out-data"].as_str().unwrap_or(""),
                    "stderr": status["data"]["err-data"].as_str().unwrap_or(""),
                }));
            }
        }
        Ok(json!({
            "status": "timeout",
            "provider": "proxmox_qemu_guest_agent",
            "node": node,
            "vmid": vmid,
            "pid": pid,
            "argv": argv,
            "timeout_seconds": timeout.as_secs(),
        }))
    }

    async fn get_json(&self, path: &str) -> Result<Value, String> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|error| format!("Proxmox API GET {path} failed: {error}"))?;
        response_json(response, path).await
    }

    async fn post_form_json(&self, path: &str, form: &[(&str, String)]) -> Result<Value, String> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", self.auth_header())
            .form(form)
            .send()
            .await
            .map_err(|error| format!("Proxmox API POST {path} failed: {error}"))?;
        response_json(response, path).await
    }

    fn auth_header(&self) -> String {
        format!("PVEAPIToken={}={}", self.client_id, self.credential)
    }
}

impl ProxmoxControlAdapter {
    /// Runtime credentials cannot provision or rotate their own successor.
    /// These explicit methods are retained as bounded rejection surfaces for
    /// callers that have not injected a rotation authority.
    pub fn issue_credential(
        &self,
        _desired: &cynic_protocol::CredentialLeaseDesiredState,
    ) -> Result<(), String> {
        Err("proxmox_credential_issuance_requires_rotation_authority".to_string())
    }

    pub fn renew_credential(
        &self,
        _desired: &cynic_protocol::CredentialLeaseDesiredState,
    ) -> Result<(), String> {
        Err("proxmox_credential_rotation_requires_rotation_authority".to_string())
    }

    pub fn revoke_credential(
        &self,
        _desired: &cynic_protocol::CredentialLeaseDesiredState,
        _generation: u64,
    ) -> Result<(), String> {
        Err("proxmox_credential_revocation_requires_rotation_authority".to_string())
    }
}

fn resolve_credential(path: &std::path::Path) -> Result<String, String> {
    for key in ["CYNIC_PROXMOX_TOKEN_SECRET", "CYNIC_PROXMOX_SECRET"] {
        if let Some(value) = optional_env(key) {
            return Ok(value);
        }
    }
    std::fs::read_to_string(path)
        .map(|value| value.trim().to_string())
        .map_err(|error| {
            format!(
                "Proxmox credential unavailable at {}: {error}",
                path.display()
            )
        })
        .and_then(|value| {
            if value.is_empty() {
                Err(format!(
                    "Proxmox credential file {} is empty",
                    path.display()
                ))
            } else {
                Ok(value)
            }
        })
}

fn observe_configuration() -> ProxmoxControlConfiguration {
    let resolved = resolve_control_configuration();
    let endpoint_source = if optional_env("CYNIC_PROXMOX_URL").is_some() {
        Some("process_environment")
    } else if resolved.endpoint.is_some() {
        Some("principal_or_installed_config")
    } else {
        None
    };
    let client_identity_source = if optional_env("CYNIC_PROXMOX_TOKEN_ID").is_some() {
        Some("process_environment")
    } else if resolved.token_id.is_some() {
        Some("principal_or_installed_config")
    } else {
        None
    };
    let credential_source = if ["CYNIC_PROXMOX_TOKEN_SECRET", "CYNIC_PROXMOX_SECRET"]
        .into_iter()
        .any(|key| optional_env(key).is_some())
    {
        Some("process_environment")
    } else if resolved.token_secret_file.is_file() {
        Some("principal_or_installed_secret")
    } else {
        None
    };
    ProxmoxControlConfiguration {
        endpoint: resolved.endpoint.is_some(),
        client_identity: resolved.token_id.is_some(),
        credential: credential_source.is_some(),
        custom_ca: resolved.ca_file.is_some(),
        identity_scope: "node_provider_control_process",
        endpoint_source,
        client_identity_source,
        credential_source,
    }
}

fn resolve_control_configuration() -> ResolvedProxmoxControl {
    let file = load_control_file();
    let registry = load_registry_provider_control();
    ResolvedProxmoxControl {
        // Non-secret provider intent comes from the runtime authority when no
        // rendered config has been installed yet.  This keeps observation
        // native and prevents the MCP projection from treating a missing
        // helper-generated TOML file as missing desired state.
        endpoint: optional_env("CYNIC_PROXMOX_URL")
            .or_else(|| file.endpoint.clone())
            .or_else(|| registry.as_ref().and_then(|value| value.endpoint.clone())),
        token_id: optional_env("CYNIC_PROXMOX_TOKEN_ID")
            .or_else(|| file.token_id.clone())
            .or_else(|| {
                registry
                    .as_ref()
                    .and_then(|value| value.token_identity.clone())
            }),
        token_secret_file: optional_env("CYNIC_PROXMOX_TOKEN_SECRET_FILE")
            .map(PathBuf::from)
            .or_else(|| file.token_secret_file.map(PathBuf::from))
            .or_else(|| {
                std::env::var("HOME").ok().and_then(|home| {
                    let p = PathBuf::from(home).join(".config/cynic/secrets/proxmox-token");
                    p.exists().then_some(p)
                })
            })
            .or_else(|| {
                registry
                    .as_ref()
                    .and_then(|value| value.credential_ref.clone())
                    .map(PathBuf::from)
            })
            .unwrap_or_else(|| PathBuf::from("/etc/cynic/secrets/proxmox-token")),
        ca_file: optional_env("CYNIC_PROXMOX_CA_FILE")
            .or(file.ca_file)
            .or_else(|| registry.as_ref().and_then(|value| value.ca_ref.clone())),
        lease_metadata_ref: optional_env("CYNIC_PROXMOX_CREDENTIAL_LEASE_METADATA_FILE")
            .or(file.credential_lease_metadata_file)
            .or_else(|| {
                registry
                    .as_ref()
                    .and_then(|value| value.credential_lease_metadata_ref.clone())
            }),
        authority_public_key_ref: optional_env("CYNIC_PROXMOX_CREDENTIAL_AUTHORITY_PUBLIC_KEY")
            .or(file.credential_authority_public_key_file)
            .or_else(|| {
                registry
                    .as_ref()
                    .and_then(|value| value.credential_authority_public_key_ref.clone())
            }),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryProviderControl {
    endpoint: Option<String>,
    token_identity: Option<String>,
    credential_ref: Option<String>,
    ca_ref: Option<String>,
    credential_lease_metadata_ref: Option<String>,
    credential_authority_public_key_ref: Option<String>,
}

fn load_registry_provider_control() -> Option<RegistryProviderControl> {
    let registry = cynic_host::runtime_registry::load_runtime_registry(None)?;
    let provider = registry.get("provider_control")?;
    if provider.get("provider").and_then(Value::as_str) != Some("proxmox") {
        return None;
    }
    serde_json::from_value(provider.clone()).ok()
}

fn load_control_file() -> ProxmoxControlFile {
    let path = optional_env("CYNIC_PROXMOX_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/etc/cynic/proxmox-control.toml"));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| toml::from_str(&content).ok())
        .unwrap_or_default()
}

fn configuration_blockers(configured: &ProxmoxControlConfiguration) -> Vec<&'static str> {
    let mut blockers = Vec::new();
    if !configured.endpoint {
        blockers.push("provider_endpoint_missing");
    }
    if !configured.client_identity {
        blockers.push("provider_token_identity_missing");
    }
    if !configured.credential {
        blockers.push("provider_token_secret_missing");
    }
    blockers
}

fn optional_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_provider_component(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("invalid Proxmox {label}"));
    }
    Ok(())
}

async fn response_json(response: reqwest::Response, path: &str) -> Result<Value, String> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("cannot read Proxmox response for {path}: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "Proxmox request {path} returned HTTP {status}: {text}"
        ));
    }
    serde_json::from_str(&text)
        .map_err(|error| format!("invalid Proxmox JSON response for {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{ProxmoxControlConfiguration, configuration_blockers, validate_provider_component};

    #[test]
    fn provider_path_components_are_bounded() {
        assert!(validate_provider_component("node", "pve-1").is_ok());
        assert!(validate_provider_component("vmid", "140").is_ok());
        assert!(validate_provider_component("node", "../pve").is_err());
        assert!(validate_provider_component("vmid", "140/agent").is_err());
    }

    #[test]
    fn provider_enrollment_blockers_are_explicit_and_secret_free() {
        let blockers = configuration_blockers(&ProxmoxControlConfiguration {
            endpoint: true,
            client_identity: false,
            credential: false,
            custom_ca: false,
            identity_scope: "node_provider_control_process",
            endpoint_source: Some("process_environment"),
            client_identity_source: None,
            credential_source: None,
        });
        assert_eq!(
            blockers,
            vec![
                "provider_token_identity_missing",
                "provider_token_secret_missing"
            ]
        );
    }

    #[test]
    fn provider_endpoint_requires_a_credential_free_https_origin() {
        let desired = |endpoint: &str| cynic_protocol::provider::ProviderControlDesiredState {
            schema: cynic_protocol::provider::PROVIDER_CONTROL_DESIRED_STATE_SCHEMA.to_string(),
            provider: "proxmox".to_string(),
            endpoint: endpoint.to_string(),
            token_identity: "cynic@pve!runtime".to_string(),
            credential_ref: "/etc/cynic/secrets/proxmox-token".to_string(),
            ca_ref: None,
            credential_lease_metadata_ref: None,
            credential_authority_public_key_ref: None,
        };
        let valid = |endpoint: &str| {
            cynic_protocol::provider::validate_provider_control_desired_state(&desired(endpoint))
                .is_empty()
        };
        assert!(valid("https://pve.internal:8006"));
        assert!(!valid("http://pve.internal:8006"));
        assert!(!valid("https://user:secret@pve.internal:8006"));
        assert!(!valid("https://pve.internal:8006/api2/json"));
    }
}
