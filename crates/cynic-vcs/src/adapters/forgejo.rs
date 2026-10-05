//! Forgejo I/O adapters — convergence planning, token lifecycle, webhooks and PRs.
//!
//! The single home for every Forgejo transport concern absorbed from the former
//! `cynic-vcs-forgejo` crate and the `forgejo`/`forgejo_port`/`forgejo_token`
//! modules of `cynic-vcs`. Pure planning (plan types, token plan, payload
//! extraction) sits beside the concrete adapters; the domain modules import
//! nothing from here.

use std::collections::HashMap;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(test)]
use cynic_network::{NETWORK_PATH_EVIDENCE_SCHEMA, NetworkPathEvidence};
use cynic_protocol::SecurityEvidenceState as ForgejoEvidenceState;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::ports::ForgejoObservationPort;
use crate::pull_request::*;
use crate::types::*;

pub fn plan_forgejo_convergence(
    desired: ForgejoDesiredState,
    observed: &ForgejoObservedState,
) -> ForgejoPlan {
    let mut actions = Vec::new();
    let mut blockers = Vec::new();

    if !observed.admin_path.admits_authenticated_application() {
        blockers.push("forgejo_authenticated_admin_path_unproven".to_string());
        actions.push(observe(
            ForgejoActionKind::ObserveApiIdentity,
            "forgejo_admin_network_path",
        ));
    }

    match observed.owner_active {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => blockers.push("forgejo_owner_state_refuted".to_string()),
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::ActivateOwner,
            true,
            "forgejo_owner_active",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_owner_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_owner_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => blockers.push("forgejo_owner_state_unknown".to_string()),
    }

    match observed.repository_exists {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_repository_state_refuted".to_string())
        }
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::CreateRepository,
            true,
            "forgejo_repository_identity",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_repository_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_repository_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_repository_state_unknown".to_string())
        }
    }

    match observed.machine_principal_ready {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::EnsureMachinePrincipal,
            true,
            "forgejo_machine_principal_restricted_non_admin",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_machine_principal_state_refuted".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_machine_principal_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_machine_principal_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_machine_principal_state_unknown".to_string())
        }
    }

    match observed.machine_key_enrolled {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_machine_key_state_refuted".to_string())
        }
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::EnrollMachineKey,
            true,
            "forgejo_machine_key_fingerprint",
        )),
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_machine_key_state_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_machine_key_state_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_machine_key_state_unknown".to_string())
        }
    }

    match observed.repository_permission_granted {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::GrantRepositoryPermission,
            true,
            "forgejo_repository_permission",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_repository_permission_refuted".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_repository_permission_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_repository_permission_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_repository_permission_unknown".to_string())
        }
    }

    if observed.ssh_host_identity != ForgejoEvidenceState::Proven {
        actions.push(observe(
            ForgejoActionKind::VerifySshHostIdentity,
            "forgejo_ssh_host_identity",
        ));
    }
    if observed.git_ssh_authenticated != ForgejoEvidenceState::Proven {
        actions.push(observe(
            ForgejoActionKind::VerifyGitSshAuthentication,
            "forgejo_git_ssh_authentication",
        ));
    }
    match observed.bootstrap_authority_revoked {
        ForgejoEvidenceState::Proven => {}
        ForgejoEvidenceState::Absent => actions.push(mutation(
            ForgejoActionKind::RevokeBootstrapAuthority,
            true,
            "forgejo_bootstrap_authority_revoked",
        )),
        ForgejoEvidenceState::Refuted => {
            blockers.push("forgejo_bootstrap_authority_still_active".to_string())
        }
        ForgejoEvidenceState::Declared => {
            blockers.push("forgejo_bootstrap_revocation_only_declared".to_string())
        }
        ForgejoEvidenceState::NotApplicable => {
            blockers.push("forgejo_bootstrap_revocation_not_applicable".to_string())
        }
        ForgejoEvidenceState::Unknown => {
            blockers.push("forgejo_bootstrap_revocation_unknown".to_string())
        }
    }

    ForgejoPlan {
        schema: FORGEJO_PLAN_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            "ready"
        } else {
            "blocked"
        }
        .to_string(),
        desired,
        actions,
        blockers,
        evidence_refs: observed.evidence_refs.clone(),
    }
}

fn observe(kind: ForgejoActionKind, proof: &str) -> ForgejoPlannedAction {
    ForgejoPlannedAction {
        kind,
        mutates: false,
        requires_human_admin_secret: false,
        proof: proof.to_string(),
    }
}

fn mutation(
    kind: ForgejoActionKind,
    requires_human_admin_secret: bool,
    proof: &str,
) -> ForgejoPlannedAction {
    ForgejoPlannedAction {
        kind,
        mutates: true,
        requires_human_admin_secret,
        proof: proof.to_string(),
    }
}

/// Build a ForgejoPlan from desired state + observer. Pure — no I/O.
pub fn build_forgejo_plan(
    desired: ForgejoDesiredState,
    observer: &dyn ForgejoObservationPort,
) -> ForgejoPlan {
    let observed = observer.observe(&desired);
    plan_forgejo_convergence(desired, &observed)
}

/// Build a ForgejoTokenSpec from the desired state + an optional token path.
pub fn forgejo_token_spec(
    desired: &ForgejoDesiredState,
    token_path: Option<&str>,
) -> ForgejoTokenSpec {
    ForgejoTokenSpec {
        schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
        principal: desired.machine_principal.clone(),
        api_url: desired
            .api_url
            .strip_suffix("/api/v1")
            .unwrap_or(&desired.api_url)
            .trim_end_matches('/')
            .to_string(),
        target_path: token_path
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/etc/cynic/secrets/forgejo-token".to_string()),
        mode: 0o640,
        scopes: vec![
            "write:repository".to_string(),
            "write:organization".to_string(),
            "write:admin".to_string(),
        ],
        expiry_seconds: Some(7_776_000),
    }
}

pub fn plan_forgejo_token_convergence(
    spec: ForgejoTokenSpec,
    observation: &ForgejoTokenObservation,
) -> ForgejoTokenPlan {
    let mut actions: Vec<ForgejoTokenPlanAction> = Vec::new();
    let mut evidence_refs: Vec<String> = Vec::new();

    if !observation.secret_present {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "secret_absent".to_string(),
        });
        evidence_refs.push(format!("token-path:{}", spec.target_path));
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.api_authenticated == ForgejoTokenAuthState::Absent {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "token_revoked_or_expired".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.api_authenticated == ForgejoTokenAuthState::Refuted {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Remove,
            reason: "token_refuted".to_string(),
        });
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Create,
            reason: "token_refuted".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    if observation.owner_root == Some(false) || observation.mode_o640 == Some(false) {
        actions.push(ForgejoTokenPlanAction {
            kind: ForgejoTokenActionKind::Remove,
            reason: "integrity_gate_failed".to_string(),
        });
        return ForgejoTokenPlan {
            status: "blocked".to_string(),
            actions,
            evidence_refs,
        };
    }

    ForgejoTokenPlan {
        status: "ready".to_string(),
        actions,
        evidence_refs,
    }
}

pub fn validate_forgejo_token_spec(spec: &ForgejoTokenSpec) -> Vec<String> {
    let mut blockers: Vec<String> = Vec::new();
    if spec.principal.is_empty() {
        blockers.push("forgejo_token_spec.principal.empty".to_string());
    }
    if spec.api_url.is_empty() {
        blockers.push("forgejo_token_spec.api_url.empty".to_string());
    }
    if spec.target_path.is_empty() {
        blockers.push("forgejo_token_spec.target_path.empty".to_string());
    }
    if spec.scopes.is_empty() {
        blockers.push("forgejo_token_spec.scopes.empty".to_string());
    }
    blockers
}

pub const FORGEJO_TOKEN_DIRECTORY: &str = "/etc/cynic/secrets";
pub const FORGEJO_TOKEN_PATH: &str = "/etc/cynic/secrets/forgejo-token";

/// The injected port the domain drives. Two adapters implement it:
/// - [`LocalForgejoTokenApiAdapter`] — runs `forgejo` CLI locally or via SSH-local Exec.
/// - [`HttpForgejoTokenApiAdapter`] — talks Forgejo REST API over HTTPS.
pub trait ForgejoTokenApiPort: Send + Sync + std::fmt::Debug {
    /// Read-only: does the token file exist / is it valid against the API?
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation;

    /// Create a new token. Returns Forgejo-side metadata only.
    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String>;

    /// Remove the token whose sha1 matches `token_sha1`.
    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String>;

    /// Read current metadata only when the adapter can bind it to the installed
    /// token through an explicit, trustworthy contract. Principal/name matching
    /// is never sufficient.
    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token metadata unavailable".to_string())
    }

    /// Prove that a previously revoked token is no longer listed remotely.
    fn is_revoked(&self, _spec: &ForgejoTokenSpec, _token_sha1: &str) -> Result<bool, String> {
        Err("token revocation read-back unavailable".to_string())
    }
}

/// Read-only file-stat observation. Never returns the token bytes.
pub fn observe_forgejo_token(spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
    if spec.target_path != FORGEJO_TOKEN_PATH {
        return ForgejoTokenObservation {
            schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
            principal: spec.principal.clone(),
            path: spec.target_path.clone(),
            secret_present: false,
            owner_root: None,
            mode_o640: None,
            api_authenticated: ForgejoTokenAuthState::Unknown,
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
            evidence_refs: Vec::new(),
        };
    }
    use std::os::unix::fs::MetadataExt;

    let target = PathBuf::from(&spec.target_path);
    let metadata = std::fs::symlink_metadata(&target);

    let mut unknowns = Vec::new();
    if spec.target_path != FORGEJO_TOKEN_PATH {
        unknowns.push("forgejo_token_target_path_noncanonical".to_string());
    }

    let (secret_present, owner_root, mode_o640) = match &metadata {
        Ok(md) => {
            if md.file_type().is_symlink() {
                unknowns.push("forgejo_token_target_is_symlink".to_string());
            }
            if !md.file_type().is_file() {
                unknowns.push("forgejo_token_target_not_regular_file".to_string());
            }
            (
                md.len() > 0,
                Some(md.uid() == 0),
                Some((md.mode() & 0o777) == spec.mode),
            )
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (false, None, None),
        Err(e) => {
            unknowns.push(format!("forgejo_token_metadata_unavailable:{e}"));
            (false, None, None)
        }
    };

    ForgejoTokenObservation {
        schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
        principal: spec.principal.clone(),
        path: spec.target_path.clone(),
        secret_present,
        owner_root,
        mode_o640,
        api_authenticated: ForgejoTokenAuthState::Unknown,
        unknowns,
        evidence_refs: Vec::new(),
    }
}

/// Local filesystem + local Exec adapter.
///
/// `create` shells out via argv (no shell composition) to `forgejo tokensAdd`.
/// The token value is read from stdout and written directly to
/// `spec.target_path`. The value is zeroized after writing.
#[derive(Debug, Clone, Copy)]
pub struct LocalForgejoTokenApiAdapter;

impl ForgejoTokenApiPort for LocalForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        observe_forgejo_token(spec)
    }

    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        let (mut token_value, metadata) = run_forgejo_tokens_add(spec)?;
        let write_result = write_protected_secret(&spec.target_path, &token_value, spec.mode);
        token_value.zeroize();
        write_result?;
        Ok(metadata)
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_token_revoke(&spec.principal, token_sha1)
    }

    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token identity is not bound to the installed secret".to_string())
    }

    fn is_revoked(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<bool, String> {
        forgejo_token_is_revoked(&spec.principal, token_sha1)
    }
}

/// HTTPS adapter. Talks Forgejo REST API directly. Used when the host has
/// network reachability to the Forgejo HTTPS endpoint. No shell involved.
#[derive(Clone)]
pub struct HttpForgejoTokenApiAdapter {
    pub api_base: String,
    pub principal: String,
    /// Runtime credential supplied by the selected adapter; never serialized
    /// or included in Debug output.
    pub authorization_token: Option<String>,
}

impl fmt::Debug for HttpForgejoTokenApiAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpForgejoTokenApiAdapter")
            .field("api_base", &self.api_base)
            .field("principal", &self.principal)
            .field(
                "authorization_token_present",
                &self.authorization_token.is_some(),
            )
            .finish()
    }
}

impl ForgejoTokenApiPort for HttpForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        let mut obs = observe_forgejo_token(spec);

        if obs.secret_present {
            match forgejo_api_validate_token(&spec.target_path, &self.api_base) {
                Ok(true) => obs.api_authenticated = ForgejoTokenAuthState::Proven,
                Ok(false) => obs.api_authenticated = ForgejoTokenAuthState::Refuted,
                Err(e) => {
                    obs.unknowns.push(format!("forgejo_api_probe_failed:{e}"));
                    obs.api_authenticated = ForgejoTokenAuthState::Unknown;
                }
            }
        } else {
            obs.api_authenticated = ForgejoTokenAuthState::Absent;
        }
        obs
    }

    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        let scopes_csv = spec.scopes.join(",");
        let resp = forgejo_http_create_token(
            &self.api_base,
            &self.principal,
            &scopes_csv,
            self.authorization_token.as_deref(),
        )?;
        let (mut token_value, metadata) = extract_api_token_material(&resp)?;
        let write_result = write_protected_secret(&spec.target_path, &token_value, spec.mode);
        token_value.zeroize();
        write_result?;
        Ok(metadata)
    }

    fn remove(&self, _spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_http_revoke_token(
            &self.api_base,
            &self.principal,
            token_sha1,
            self.authorization_token.as_deref(),
        )
    }

    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token identity is not bound to the installed secret".to_string())
    }

    fn is_revoked(&self, _spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<bool, String> {
        forgejo_http_token_is_revoked(
            &self.api_base,
            &self.principal,
            token_sha1,
            self.authorization_token.as_deref(),
        )
    }
}

/// Adapter used when runtime reachability and credential injection have not
/// been proven. It never probes or mutates, so the surface fails closed.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableForgejoTokenApiAdapter;

impl ForgejoTokenApiPort for UnavailableForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        let mut observation = observe_forgejo_token(spec);
        observation.api_authenticated = ForgejoTokenAuthState::Unknown;
        observation
            .unknowns
            .push("forgejo_token_adapter_selection_unproven".to_string());
        observation
    }

    fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("Forgejo token adapter selection is unproven".to_string())
    }

    fn remove(&self, _spec: &ForgejoTokenSpec, _token_sha1: &str) -> Result<(), String> {
        Err("Forgejo token adapter selection is unproven".to_string())
    }
}

/// Boxed port handle so the runtime can swap adapters (local-exec vs HTTP).
pub type BoxedForgejoTokenPort = Box<dyn ForgejoTokenApiPort>;

/// Spec enriched with the adapter to drive. Decoupled from `ForgejoTokenSpec`
/// so the protocol-level types stay transport-agnostic.
#[derive(Debug)]
pub struct ForgejoTokenRuntimeSpec {
    pub token_spec: ForgejoTokenSpec,
    pub adapter: BoxedForgejoTokenPort,
}

/// Builder that derives the canonical token spec from runtime config.
pub struct DefaultForgejoTokenSpec;

impl DefaultForgejoTokenSpec {
    pub fn default_with_principal(principal: String) -> ForgejoTokenSpec {
        let api_url = std::env::var("CYNIC_FORGEJO_API_URL")
            .or_else(|_| std::env::var("FORGEJO_API_URL"))
            .or_else(|_| std::env::var("FORGEJO_URL"))
            .unwrap_or_else(|_| "http://forgejo.synapse.cynic:3000".to_string());
        ForgejoTokenSpec {
            schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
            principal,
            api_url,
            scopes: vec![
                "read:organization".to_string(),
                "write:organization".to_string(),
                "write:repository".to_string(),
            ],
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            mode: 0o640,
            expiry_seconds: None,
        }
    }

    pub fn with_token_path(token_path: &str) -> ForgejoTokenSpec {
        let mut spec = Self::default_with_principal("cynic-cortex".to_string());
        spec.target_path = token_path.to_string();
        spec
    }
}

impl Default for ForgejoTokenRuntimeSpec {
    fn default() -> Self {
        Self {
            token_spec: DefaultForgejoTokenSpec::default_with_principal("cynic-cortex".to_string()),
            adapter: Box::new(UnavailableForgejoTokenApiAdapter),
        }
    }
}

/// Observe the token through the injected port + emit a convergence plan.
/// This is the runtime entry point for `cynic forgejo token-status`.
pub fn observe_forgejo_token_runtime(spec: &ForgejoTokenRuntimeSpec) -> ForgejoTokenObservation {
    spec.adapter.observe(&spec.token_spec)
}

/// Provision (create/rotate) the token through the injected port and emit a
/// read-back proof report. Follows the `observe → apply → read-back` coherence
/// pattern. This is the runtime entry point for `cynic forgejo token-create|rotate`.
pub fn provision_forgejo_token_apply(
    spec: &ForgejoTokenRuntimeSpec,
) -> ForgejoTokenProvisionReport {
    provision_forgejo_token(&spec.token_spec, &*spec.adapter)
}
pub fn provision_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    if spec.target_path != FORGEJO_TOKEN_PATH {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let observation = port.observe(spec);

    if observation.api_authenticated == ForgejoTokenAuthState::Proven {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "ready".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; nothing to roll back".to_string(),
            unknowns: Vec::new(),
        };
    }

    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_provision_read_back_not_converged".to_string());
            }
            let sha1_for_rollback = metadata.sha1.clone();
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
                status: if converged {
                    "completed".to_string()
                } else {
                    "blocked".to_string()
                },
                principal: spec.principal.clone(),
                metadata: if converged && metadata.sha1 != "<unavailable>" {
                    Some(metadata.clone())
                } else {
                    None
                },
                token_metadata: if converged && metadata.sha1 != "<unavailable>" {
                    Some(metadata)
                } else {
                    None
                },
                action_taken: if converged {
                    "created".to_string()
                } else {
                    "blocked".to_string()
                },
                rollback_intent: format!(
                    "revoke Forgejo token {} for principal '{}' via the same port",
                    sha1_for_rollback, spec.principal
                ),
                unknowns,
            }
        }
        Err(e) => ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation due to error; nothing to roll back".to_string(),
            unknowns: vec![format!("forgejo_token_create_failed:{e}")],
        },
    }
}

/// Rotate by revoking the exact current token, creating its replacement, and
/// requiring a fresh observation proving the replacement.
pub fn rotate_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    if spec.target_path != FORGEJO_TOKEN_PATH {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            principal: spec.principal.clone(),
            metadata: None,
            token_metadata: None,
            action_taken: "none".to_string(),
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let before = port.observe(spec);
    let old = match port.current_token(spec) {
        Ok(metadata) if validate_token_identity(&metadata.sha1).is_ok() => metadata,
        _ => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_current_metadata_unavailable",
            );
        }
    };
    if let Err(error) = port.remove(spec, &old.sha1) {
        return blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_revoke_failed:{error}"),
        );
    }
    match port.is_revoked(spec, &old.sha1) {
        Ok(true) => {}
        Ok(false) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_revocation_read_back_refuted",
            );
        }
        Err(error) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                &format!("forgejo_token_revocation_read_back_unknown:{error}"),
            );
        }
    }
    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_rotation_read_back_not_converged".to_string());
            }
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
                status: if converged {
                    "completed".to_string()
                } else {
                    "blocked".to_string()
                },
                principal: spec.principal.clone(),
                metadata: if converged {
                    Some(metadata.clone())
                } else {
                    None
                },
                token_metadata: if converged { Some(metadata) } else { None },
                action_taken: if converged {
                    "rotated".to_string()
                } else {
                    "blocked".to_string()
                },
                rollback_intent: format!(
                    "revoke replacement token for principal '{}' via the same port",
                    spec.principal
                ),
                unknowns,
            }
        }
        Err(error) => blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_create_failed_after_revoke:{error}"),
        ),
    }
}

fn blocked_rotation_report(
    spec: &ForgejoTokenSpec,
    _port: &dyn ForgejoTokenApiPort,
    _before: ForgejoTokenObservation,
    reason: &str,
) -> ForgejoTokenProvisionReport {
    ForgejoTokenProvisionReport {
        schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        principal: spec.principal.clone(),
        metadata: None,
        token_metadata: None,
        action_taken: "none".to_string(),
        rollback_intent: "rotation blocked; inspect revocation and replacement state".to_string(),
        unknowns: vec![reason.to_string()],
    }
}

// ── internal helpers (local-exec + http) ──────────────────────────────

fn run_forgejo_tokens_add(
    spec: &ForgejoTokenSpec,
) -> Result<(String, ForgejoTokenMetadata), String> {
    let mut cmd = Command::new("forgejo");
    cmd.args([
        "tokensAdd",
        &spec.principal,
        "--scopes",
        &spec.scopes.join(","),
    ])
    .arg("--format")
    .arg("json");

    if let Some(expiry) = spec.expiry_seconds {
        cmd.args(["--expiry", &format!("{}s", expiry)]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("forgejo tokensAdd spawn failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "forgejo tokensAdd failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Forgejo tokensAdd --format json emits {"sha1":"...","token":"..."}
    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("forgejo JSON parse failed: {e}"))?;
    let token = value
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "forgejo tokensAdd did not return a token field".to_string())?;
    let metadata = extract_token_metadata(&value)?;
    Ok((token.to_string(), metadata))
}

fn forgejo_token_is_revoked(principal: &str, token_sha1: &str) -> Result<bool, String> {
    let output = Command::new("forgejo")
        .args(["tokens", principal, "--format", "json"])
        .output()
        .map_err(|e| format!("forgejo token list spawn failed: {e}"))?;
    if !output.status.success() {
        return Err("forgejo token list failed during revocation read-back".to_string());
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("forgejo token list JSON parse failed: {e}"))?;
    let entries = value
        .as_array()
        .ok_or_else(|| "forgejo token list returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
}

fn forgejo_token_revoke(principal: &str, sha1: &str) -> Result<(), String> {
    let output = Command::new("forgejo")
        .args(["tokensDelete", principal])
        .arg(sha1)
        .output()
        .map_err(|e| format!("forgejo tokensDelete spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "forgejo tokensDelete failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn forgejo_api_validate_token(token_path: &str, api_base: &str) -> Result<bool, String> {
    if token_path != FORGEJO_TOKEN_PATH {
        return Err("token API probe requires the canonical token path".to_string());
    }
    let mut token =
        std::fs::read_to_string(FORGEJO_TOKEN_PATH).map_err(|e| format!("read token file: {e}"))?;
    let token_value = token.trim().to_string();
    token.zeroize();
    let client = http_client()?;
    let result = client
        .get(format_api_url(api_base, "/user"))
        .header("Authorization", format!("token {token_value}"))
        .send()
        .map(|resp| resp.status().is_success())
        .map_err(|e| format!("API probe request failed: {e}"));
    drop(token_value);
    result
}

fn forgejo_http_create_token(
    api_base: &str,
    principal: &str,
    scopes: &str,
    authorization_token: Option<&str>,
) -> Result<serde_json::Value, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let body = serde_json::json!({ "name": principal, "scopes": scopes });
    let resp = client
        .post(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .json(&body)
        .send()
        .map_err(|e| format!("token creation request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token creation HTTP {}", resp.status()));
    }
    resp.json()
        .map_err(|e| format!("token creation JSON parse failed: {e}"))
}

fn forgejo_http_token_is_revoked(
    api_base: &str,
    principal: &str,
    token_sha1: &str,
    authorization_token: Option<&str>,
) -> Result<bool, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .get(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .send()
        .map_err(|e| format!("token revocation read-back request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revocation read-back HTTP {}", resp.status()));
    }
    let tokens: serde_json::Value = resp
        .json()
        .map_err(|e| format!("token revocation read-back JSON parse failed: {e}"))?;
    let entries = tokens
        .as_array()
        .ok_or_else(|| "token revocation read-back returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
}

fn forgejo_http_revoke_token(
    api_base: &str,
    principal: &str,
    sha1: &str,
    authorization_token: Option<&str>,
) -> Result<(), String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .delete(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens/{sha1}"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .send()
        .map_err(|e| format!("token revoke request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revoke HTTP {}", resp.status()));
    }
    Ok(())
}

fn extract_token_from_api_response(resp: &serde_json::Value) -> Result<String, String> {
    resp.get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "API response missing token field".to_string())
}

fn extract_api_token_material(
    resp: &serde_json::Value,
) -> Result<(String, ForgejoTokenMetadata), String> {
    let mut token = extract_token_from_api_response(resp)?;
    let metadata = match extract_token_metadata(resp) {
        Ok(metadata) => metadata,
        Err(error) => {
            token.zeroize();
            return Err(error);
        }
    };
    Ok((token, metadata))
}

fn validate_token_identity(sha1: &str) -> Result<(), String> {
    if sha1.len() != 40 {
        return Err("Forgejo token identity must be exactly 40 hexadecimal characters".to_string());
    }
    if !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Forgejo token identity must be hexadecimal".to_string());
    }
    Ok(())
}

fn extract_token_metadata(resp: &serde_json::Value) -> Result<ForgejoTokenMetadata, String> {
    Ok(ForgejoTokenMetadata {
        token_id: resp
            .get("id")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "token response missing exact id".to_string())?,
        name: resp
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "token response missing exact name".to_string())?
            .to_string(),
        sha1: resp
            .get("sha1")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "token response missing exact sha1".to_string())?
            .to_string(),
    })
}

fn required_authorization(token: Option<&str>) -> Result<&str, String> {
    token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Forgejo HTTP mutation requires Authorization token".to_string())
}

fn format_api_url(api_base: &str, path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let base = base.strip_suffix("/api/v1").unwrap_or(base);
    format!("{base}/api/v1/{}", path.trim_start_matches('/'))
}

fn write_protected_secret(target: &str, token: &str, mode: u32) -> Result<(), String> {
    if target != FORGEJO_TOKEN_PATH {
        return Err(format!(
            "secret target must be canonical path {FORGEJO_TOKEN_PATH}"
        ));
    }
    let path = PathBuf::from(target);
    let parent = path
        .parent()
        .ok_or_else(|| "secret target has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("secret directory creation failed: {e}"))?;
    if parent
        .canonicalize()
        .map_err(|e| format!("secret directory canonicalization failed: {e}"))?
        != PathBuf::from(FORGEJO_TOKEN_DIRECTORY)
    {
        return Err("secret directory must not resolve through a symlink".to_string());
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err("secret target symlink rejected".to_string());
        }
        if !metadata.file_type().is_file() {
            return Err("secret target is not a regular file".to_string());
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_nanos();
    let temporary = parent.join(format!(".forgejo-token.{}.{}", std::process::id(), nonce));
    let mut file = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .custom_flags(0o400000)
        .open(&temporary)
    {
        Ok(file) => file,
        Err(e) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(format!("secret temporary file write failed: {e}"));
        }
    };
    if let Err(e) = file.write_all(token.as_bytes()) {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret write_all failed: {e}"));
    }
    if let Err(e) = file.sync_all() {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret sync failed: {e}"));
    }
    drop(file);
    if let Err(e) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret atomic rename failed: {e}"));
    }
    Ok(())
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))
}

/// Forgejo webhook payload (minimal — only what we need).
#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPayload {
    #[serde(rename = "ref")]
    pub branch_ref: Option<String>,
    pub after: Option<String>,
    pub pull_request: Option<ForgejoPullRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPullRequest {
    pub head: ForgejoPrHead,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPrHead {
    pub sha: String,
}

/// Extract exact source commit (validated 40-char hex SHA) from payload.
pub fn extract_source_commit(payload: &ForgejoPayload) -> Option<String> {
    if let Some(pr) = &payload.pull_request {
        let sha = pr.head.sha.trim();
        if validate_source_commit(sha) {
            return Some(sha.to_string());
        }
    }
    if let Some(after) = &payload.after {
        let sha = after.trim();
        if validate_source_commit(sha) {
            return Some(sha.to_string());
        }
    }
    None
}

/// Validate a git commit SHA: exactly 40 hex characters.
pub fn validate_source_commit(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Extract branch name from ref.
pub fn extract_branch(payload: &ForgejoPayload) -> Option<String> {
    payload
        .branch_ref
        .as_ref()
        .map(|r| r.replace("refs/heads/", ""))
}

/// Result of processing a forgejo webhook.
#[derive(Debug, Clone, Serialize)]
pub enum ForgejoHandlerResult {
    /// Build triggered, contains workload id.
    Triggered {
        workload_id: String,
        source_commit: String,
    },
    /// Skipped (no commit, wrong branch).
    Skipped { reason: String },
}

/// Configuration for the forgejo webhook handler.
#[derive(Debug, Clone)]
pub struct ForgejoHandlerConfig {
    /// Only build this branch (None = any).
    pub require_branch: Option<String>,
    /// Default build phase.
    pub default_phase: String,
}

impl Default for ForgejoHandlerConfig {
    fn default() -> Self {
        Self {
            require_branch: Some("main".to_string()),
            default_phase: "check".to_string(),
        }
    }
}

/// Process a forgejo webhook payload.
pub fn handle_forgejo_payload(
    payload: &ForgejoPayload,
    config: &ForgejoHandlerConfig,
) -> ForgejoHandlerResult {
    let source_commit = match extract_source_commit(payload) {
        Some(sha) => sha,
        None => {
            return ForgejoHandlerResult::Skipped {
                reason: "no_commit_sha_found".to_string(),
            };
        }
    };

    if let Some(required) = &config.require_branch {
        let branch = extract_branch(payload);
        if branch.as_deref() != Some(required) {
            return ForgejoHandlerResult::Skipped {
                reason: format!("branch_mismatch: required={required}"),
            };
        }
    }

    let workload_id = format!("forgejo-{}", &source_commit[..24]);

    ForgejoHandlerResult::Triggered {
        workload_id,
        source_commit,
    }
}

/// Configuration for the Forgejo webhook server.
#[derive(Debug, Clone)]
pub struct WebhookServerConfig {
    /// Address to bind (e.g. "127.0.0.1:9147").
    pub bind: String,
    /// Node builder webhook URL (e.g. "http://127.0.0.1:3030/webhook/forgejo").
    /// Empty means forwarding is unconfigured: pushes validate but never
    /// forward (explicit blocked result, never a guessed target).
    pub node_webhook_url: String,
    /// Shared Forgejo webhook secret. When set, the
    /// `X-Forgejo-Signature` header (hex HMAC-SHA256 over the raw body)
    /// must verify or the push is rejected. When unset, pushes are
    /// accepted without signature proof and reported as such.
    /// Load from `CYNIC_WEBHOOK_SECRET`, never from CLI flags.
    pub webhook_secret: Option<String>,
}

/// Result of a webhook-forwarded build request.
#[derive(Debug, Clone, Serialize)]
pub struct WebhookTriggerResult {
    pub status: String,
    pub source_commit: Option<String>,
    pub workload_id: Option<String>,
    pub build_triggered: bool,
    pub build_proof: Option<String>,
    pub signature_verified: bool,
    pub error: Option<String>,
}

/// Spawn the webhook server thread.
pub fn spawn_webhook_server(
    config: WebhookServerConfig,
) -> std::io::Result<thread::JoinHandle<()>> {
    let listener = TcpListener::bind(&config.bind)?;
    listener.set_nonblocking(false)?;

    let handle = thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let result = handle_webhook_request(&mut stream, &config);
                respond_json(&mut stream, &result);
            }
        }
    });

    Ok(handle)
}

/// A parsed HTTP request: lowercase header map plus the raw body bytes.
/// The raw bytes are retained so the HMAC verifies exactly what Forgejo sent.
struct ParsedRequest {
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<ParsedRequest, String> {
    let mut buf = [0u8; 8192];
    let mut raw: Vec<u8> = Vec::new();
    // Bounded read: headers plus at most 1 MiB of body.
    while raw.len() < 1_100_000 {
        let n = match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return Err("failed to read request".to_string()),
        };
        raw.extend_from_slice(&buf[..n]);
        if let Some(idx) = find_header_end(&raw) {
            let body_len = content_length(&raw[..idx]);
            if raw.len() >= idx + body_len {
                let head = String::from_utf8_lossy(&raw[..idx]).into_owned();
                let body = raw[idx..idx + body_len.min(raw.len() - idx)].to_vec();
                return Ok(ParsedRequest {
                    headers: parse_headers(&head),
                    body,
                });
            }
        }
    }
    Err("incomplete HTTP request".to_string())
}

fn find_header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn content_length(head: &[u8]) -> usize {
    let text = String::from_utf8_lossy(head);
    text.lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn parse_headers(head: &str) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for line in head.lines().skip(1) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    headers
}

fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let presented = match hex::decode(signature.trim()) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&presented).is_ok()
}

/// Handle a single HTTP POST request: verify → validate → forward → report.
fn handle_webhook_request(
    stream: &mut TcpStream,
    config: &WebhookServerConfig,
) -> WebhookTriggerResult {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => return error_result(&error),
    };

    let presented_signature = request.headers.get("x-forgejo-signature").cloned();
    let signature_verified = match config.webhook_secret.as_deref() {
        Some(secret) => {
            let presented = presented_signature.as_deref().unwrap_or("");
            if !verify_signature(secret, &request.body, presented) {
                return WebhookTriggerResult {
                    status: "rejected".to_string(),
                    source_commit: None,
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified: false,
                    error: Some("invalid webhook signature".to_string()),
                };
            }
            true
        }
        None => false,
    };

    // Parse Forgejo payload from the exact bytes that were verified.
    let body_text = match std::str::from_utf8(&request.body) {
        Ok(text) => text,
        Err(_) => return error_result("invalid UTF-8 in request body"),
    };
    let payload: crate::adapters::forgejo::ForgejoPayload = match serde_json::from_str(body_text) {
        Ok(payload) => payload,
        Err(error) => {
            return error_result(&format!("failed to parse Forgejo payload: {error}"));
        }
    };

    // Handle via existing forgejo webhook handler
    let handler_result = handle_forgejo_payload(&payload, &ForgejoHandlerConfig::default());

    match handler_result {
        crate::adapters::forgejo::ForgejoHandlerResult::Triggered {
            workload_id: _,
            source_commit,
        } => {
            // Fail-closed: never forward with invalid SHA
            if !validate_source_commit(&source_commit) {
                return WebhookTriggerResult {
                    status: "rejected".to_string(),
                    source_commit: Some(source_commit),
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified,
                    error: Some("invalid source commit SHA — rejecting build".to_string()),
                };
            }

            if config.node_webhook_url.trim().is_empty() {
                return WebhookTriggerResult {
                    status: "blocked".to_string(),
                    source_commit: Some(source_commit),
                    workload_id: None,
                    build_triggered: false,
                    build_proof: None,
                    signature_verified,
                    error: Some(
                        "node webhook URL unconfigured — refusing to guess a target".to_string(),
                    ),
                };
            }

            forward_to_node(
                config,
                &request.body,
                &source_commit,
                presented_signature.as_deref(),
                signature_verified,
            )
        }
        crate::adapters::forgejo::ForgejoHandlerResult::Skipped { reason } => {
            WebhookTriggerResult {
                status: "skipped".to_string(),
                source_commit: None,
                workload_id: None,
                build_triggered: false,
                build_proof: None,
                signature_verified,
                error: Some(reason),
            }
        }
    }
}

/// Forward the verified raw payload to the node builder webhook, which owns
/// admission, durable persistence, and scheduling. The signature header is
/// passed through untouched so the node verifies the same bytes.
fn forward_to_node(
    config: &WebhookServerConfig,
    raw_body: &[u8],
    source_commit: &str,
    presented_signature: Option<&str>,
    signature_verified: bool,
) -> WebhookTriggerResult {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build();
    let client = match client {
        Ok(client) => client,
        Err(error) => {
            return error_result(&format!("webhook forward client failed: {error}"));
        }
    };
    let mut forward = client
        .post(config.node_webhook_url.as_str())
        .header("Content-Type", "application/json")
        .body(raw_body.to_vec());
    if let Some(signature) = presented_signature {
        forward = forward.header("X-Forgejo-Signature", signature.to_string());
    }
    let response = forward.send();
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            return error_result(&format!("webhook forward failed: {error}"));
        }
    };
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        return WebhookTriggerResult {
            status: "trigger_failed".to_string(),
            source_commit: Some(source_commit.to_string()),
            workload_id: None,
            build_triggered: false,
            build_proof: Some(text.clone()),
            signature_verified,
            error: Some(format!("node webhook refused: HTTP {}", status.as_u16())),
        };
    }
    let proof: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    let workload_id = proof
        .get("workload_id")
        .and_then(|id| id.as_str())
        .map(String::from);
    WebhookTriggerResult {
        status: "triggered".to_string(),
        source_commit: Some(source_commit.to_string()),
        workload_id,
        build_triggered: true,
        build_proof: Some(text),
        signature_verified,
        error: None,
    }
}

/// Helper for the error path.
fn error_result(msg: &str) -> WebhookTriggerResult {
    WebhookTriggerResult {
        status: "error".to_string(),
        source_commit: None,
        workload_id: None,
        build_triggered: false,
        build_proof: None,
        signature_verified: false,
        error: Some(msg.to_string()),
    }
}

/// Write a minimal HTTP/1.0 JSON response.
fn respond_json(stream: &mut TcpStream, result: &WebhookTriggerResult) {
    let body = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());

    let response = format!(
        "HTTP/1.0 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        body.len(),
        body,
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub struct ForgejoPrAdapter {
    pub api_base: String,
    pub owner: String,
    pub repo: String,
    pub token: Option<String>,
}

impl ForgejoPrAdapter {
    pub fn new(api_base: String, owner: String, repo: String, token: Option<String>) -> Self {
        Self {
            api_base,
            owner,
            repo,
            token,
        }
    }

    fn auth_header(&self) -> Option<String> {
        self.token
            .as_ref()
            .map(|t| format!("Authorization: token {t}"))
    }

    fn curl_get(&self, url: &str) -> Result<serde_json::Value, String> {
        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-f", "-X", "GET", url]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "GET {} failed ({}): {}",
                url,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid JSON: {e}"))
    }

    fn curl_post(&self, url: &str, body: &str) -> Result<serde_json::Value, String> {
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-X",
            "POST",
            url,
            "-H",
            "Content-Type: application/json",
            "-d",
            body,
        ]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "POST {} failed ({}): {}",
                url,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid JSON: {e}"))
    }
}

fn parse_pr(item: &serde_json::Value) -> PullRequest {
    let str_path = |path: &[&str]| -> String {
        let mut v = item;
        for &k in path {
            v = &v[k];
        }
        v.as_str().unwrap_or_default().to_string()
    };
    PullRequest {
        number: item["number"].as_u64().unwrap_or_default(),
        title: str_path(&["title"]),
        state: match item["state"].as_str().unwrap_or("open") {
            "closed" => PrState::Closed,
            _ => PrState::Open,
        },
        head_ref: str_path(&["head", "ref"]),
        head_sha: str_path(&["head", "sha"]),
        base_ref: str_path(&["base", "ref"]),
        base_sha: str_path(&["base", "sha"]),
        mergeable: item["mergeable"].as_bool().unwrap_or(true),
        has_conflicts: item["has_conflicts"].as_bool().unwrap_or(false),
        html_url: str_path(&["html_url"]),
    }
}

impl PrPort for ForgejoPrAdapter {
    fn list(&self, state: &str) -> Result<Vec<PullRequest>, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls?state={}&limit=50",
            self.api_base, self.owner, self.repo, state
        );
        let json = self.curl_get(&url)?;
        let items = json.as_array().ok_or("expected JSON array")?;
        Ok(items.iter().map(parse_pr).collect())
    }

    fn open(
        &self,
        title: &str,
        head: &str,
        base: &str,
        body: &str,
    ) -> Result<PrCreateProof, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls",
            self.api_base, self.owner, self.repo
        );
        let payload = serde_json::json!({"title": title, "head": head, "base": base, "body": body});
        let res = self.curl_post(&url, &payload.to_string())?;
        let pr_number = res["number"]
            .as_u64()
            .ok_or("missing PR number in response")?;
        let html_url = res["html_url"].as_str().unwrap_or_default().to_string();
        Ok(PrCreateProof {
            schema: PR_CREATE_PROOF_SCHEMA,
            pr_number,
            html_url,
            head_ref: head.to_string(),
            base_ref: base.to_string(),
        })
    }

    fn merge(&self, number: u64, style: &str) -> Result<PrMergeProof, String> {
        // Forgejo merge returns 204 No Content — discard body, check exit code only.
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}/merge",
            self.api_base, self.owner, self.repo, number
        );
        let payload = serde_json::json!({"Do": style});
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-o",
            "/dev/null",
            "-X",
            "POST",
            &url,
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload.to_string(),
        ]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "merge PR#{number} failed ({}): {}",
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(PrMergeProof {
            schema: PR_MERGE_PROOF_SCHEMA,
            pr_number: number,
            merged: true,
            message: format!("PR #{number} merged via Forgejo (style: {style})"),
        })
    }
}

#[cfg(test)]
mod convergence_tests {
    use super::*;

    fn desired() -> ForgejoDesiredState {
        ForgejoDesiredState {
            instance_id: "forgejo".into(),
            owner: "cynic".into(),
            repository: "CYNIC".into(),
            api_url: "http://forgejo:3000/api/v1".into(),
            git_ssh_url: "ssh://git@forgejo:22/cynic/CYNIC.git".into(),
            machine_principal: "cynic-cortex".into(),
            repository_permission: ForgejoRepositoryPermission::Write,
            machine_key_fingerprint: "SHA256:machine".into(),
        }
    }

    #[test]
    fn absent_resources_become_separate_explicit_mutations() {
        let observed = ForgejoObservedState {
            admin_path: NetworkPathEvidence {
                schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
                id: "forgejo-admin".into(),
                service_ref: "forgejo".into(),
                name_resolution: ForgejoEvidenceState::Proven,
                network_route: ForgejoEvidenceState::Proven,
                transport_reachability: ForgejoEvidenceState::Proven,
                confidentiality: ForgejoEvidenceState::Proven,
                peer_identity: ForgejoEvidenceState::Proven,
                application_identity: ForgejoEvidenceState::Proven,
                resolved_addresses: Vec::new(),
                evidence_refs: vec!["admin-path.json".into()],
            },
            owner_active: ForgejoEvidenceState::Proven,
            repository_exists: ForgejoEvidenceState::Absent,
            machine_principal_ready: ForgejoEvidenceState::Absent,
            machine_key_enrolled: ForgejoEvidenceState::Absent,
            repository_permission_granted: ForgejoEvidenceState::Absent,
            ssh_host_identity: ForgejoEvidenceState::Proven,
            git_ssh_authenticated: ForgejoEvidenceState::Absent,
            bootstrap_authority_revoked: ForgejoEvidenceState::Absent,
            evidence_refs: vec!["probe.json".into()],
        };

        let plan = plan_forgejo_convergence(desired(), &observed);
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.actions.len(), 6);
        assert_eq!(plan.actions[0].kind, ForgejoActionKind::CreateRepository);
        assert_eq!(
            plan.actions[1].kind,
            ForgejoActionKind::EnsureMachinePrincipal
        );
        assert_eq!(
            plan.actions[4].kind,
            ForgejoActionKind::VerifyGitSshAuthentication
        );
        assert!(plan.actions[0].requires_human_admin_secret);
        assert!(!plan.actions[4].mutates);
    }

    #[test]
    fn unknown_resource_state_blocks_mutation_planning() {
        let observed = ForgejoObservedState {
            admin_path: NetworkPathEvidence {
                schema: NETWORK_PATH_EVIDENCE_SCHEMA.to_string(),
                id: "forgejo-admin".into(),
                service_ref: "forgejo".into(),
                name_resolution: ForgejoEvidenceState::Proven,
                network_route: ForgejoEvidenceState::Proven,
                transport_reachability: ForgejoEvidenceState::Proven,
                confidentiality: ForgejoEvidenceState::Absent,
                peer_identity: ForgejoEvidenceState::Unknown,
                application_identity: ForgejoEvidenceState::Unknown,
                resolved_addresses: Vec::new(),
                evidence_refs: Vec::new(),
            },
            owner_active: ForgejoEvidenceState::Unknown,
            repository_exists: ForgejoEvidenceState::Unknown,
            machine_principal_ready: ForgejoEvidenceState::Unknown,
            machine_key_enrolled: ForgejoEvidenceState::Unknown,
            repository_permission_granted: ForgejoEvidenceState::Unknown,
            ssh_host_identity: ForgejoEvidenceState::Unknown,
            git_ssh_authenticated: ForgejoEvidenceState::Unknown,
            bootstrap_authority_revoked: ForgejoEvidenceState::Unknown,
            evidence_refs: Vec::new(),
        };

        let plan = plan_forgejo_convergence(desired(), &observed);
        assert_eq!(plan.status, "blocked");
        assert_eq!(plan.blockers.len(), 7);
        assert!(plan.actions.iter().all(|action| !action.mutates));
    }
}
#[cfg(test)]
mod token_adapter_tests {
    use super::*;

    #[test]
    fn observe_uses_path_from_spec() {
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = observe_forgejo_token(&spec);
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Unknown);
        assert_eq!(obs.path, "/tmp/nonexistent-token-xyz");
    }

    #[test]
    fn provision_skips_when_token_already_valid() {
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                let mut obs = observe_forgejo_token(spec);
                obs.api_authenticated = ForgejoTokenAuthState::Proven;
                obs
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("should not be called".into())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("should not be called".into())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = provision_forgejo_token(&spec, &StubPort);
        assert_eq!(report.status, "ready");
        assert_eq!(report.action_taken, "none");
        assert!(report.metadata.is_none());
    }

    #[test]
    fn concrete_adapters_fail_closed_without_installed_identity_binding() {
        let spec = ForgejoTokenSpec::default();
        assert!(LocalForgejoTokenApiAdapter.current_token(&spec).is_err());
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.example".to_string(),
            principal: spec.principal.clone(),
            authorization_token: None,
        };
        assert!(http.current_token(&spec).is_err());
    }

    #[test]
    fn http_adapter_absent_when_file_missing() {
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.synapse.cynic/api/v1".to_string(),
            principal: "cynic-cortex".to_string(),
            authorization_token: None,
        };
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-http-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = http.observe(&spec);
        // File missing → secret_present false → ApiAuthenticated::Absent (no API call).
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Absent);
    }

    #[test]
    fn http_mutations_require_auth_and_normalize_api_base() {
        assert!(required_authorization(None).is_err());
        assert_eq!(
            format_api_url("https://forgejo.example/api/v1/", "/user"),
            "https://forgejo.example/api/v1/user"
        );
    }

    #[test]
    fn secret_writer_rejects_noncanonical_targets_before_io() {
        let error = write_protected_secret("/tmp/arbitrary-token", "secret", 0o640)
            .expect_err("arbitrary token paths must be rejected");
        assert!(error.contains("canonical path"));
    }

    #[test]
    fn noncanonical_paths_blocked_before_port_interaction() {
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("must not create".to_string())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("must not remove".to_string())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: "/tmp/noncanonical-token".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let provision = provision_forgejo_token(&spec, &StubPort);
        let rotation = rotate_forgejo_token(&spec, &StubPort);
        assert_eq!(provision.status, "blocked");
        assert_eq!(provision.action_taken, "none");
        assert_eq!(rotation.status, "blocked");
        assert_eq!(rotation.action_taken, "none");
    }

    #[test]
    fn api_material_requires_metadata_before_secret_write() {
        let response = serde_json::json!({"token": "raw-secret"});
        let error = extract_api_token_material(&response).expect_err("metadata is required first");
        assert!(error.contains("missing exact id"));
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_before_remove() {
        #[derive(Debug)]
        struct InvalidIdentityPort;
        impl ForgejoTokenApiPort for InvalidIdentityPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                panic!("create must not run")
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                panic!("remove must not run for an invalid identity")
            }
            fn current_token(
                &self,
                _spec: &ForgejoTokenSpec,
            ) -> Result<ForgejoTokenMetadata, String> {
                Ok(ForgejoTokenMetadata {
                    token_id: 1,
                    name: "cynic".to_string(),
                    sha1: "<unavailable>".to_string(),
                })
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = rotate_forgejo_token(&spec, &InvalidIdentityPort);
        assert_eq!(report.status, "blocked");
        assert_eq!(report.action_taken, "none");
        assert_eq!(
            report.unknowns[0],
            "forgejo_token_current_metadata_unavailable"
        );
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_and_non_hex() {
        assert!(validate_token_identity("").is_err());
        assert!(validate_token_identity("<unavailable>").is_err());
        assert!(validate_token_identity(&"a".repeat(39)).is_err());
        assert!(validate_token_identity(&"g".repeat(40)).is_err());
        assert!(validate_token_identity(&"a".repeat(40)).is_ok());
    }

    #[test]
    fn runtime_default_is_unavailable() {
        let runtime = ForgejoTokenRuntimeSpec::default();
        let observation = observe_forgejo_token_runtime(&runtime);
        assert!(
            observation
                .unknowns
                .iter()
                .any(|unknown| unknown == "forgejo_token_adapter_selection_unproven")
        );
        assert!(runtime.adapter.create(&runtime.token_spec).is_err());
    }

    #[test]
    fn default_spec_uses_canonical_path() {
        let spec = DefaultForgejoTokenSpec::with_token_path("/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.target_path, "/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.principal, "cynic-cortex");
        assert_eq!(spec.mode, 0o640);
    }
}
#[cfg(test)]
mod webhook_tests {
    use super::*;

    #[test]
    fn extract_commit_from_push() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/main".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        assert_eq!(extract_source_commit(&payload), Some("a".repeat(40)));
    }

    #[test]
    fn extract_commit_from_pr() {
        let payload = ForgejoPayload {
            branch_ref: None,
            after: None,
            pull_request: Some(ForgejoPullRequest {
                head: ForgejoPrHead {
                    sha: "b".repeat(40),
                },
            }),
        };
        assert_eq!(extract_source_commit(&payload), Some("b".repeat(40)));
    }

    #[test]
    fn skip_invalid_commit() {
        let payload = ForgejoPayload {
            branch_ref: None,
            after: Some("short".to_string()),
            pull_request: None,
        };
        assert_eq!(extract_source_commit(&payload), None);
    }

    #[test]
    fn handler_skips_wrong_branch() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/dev".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        let config = ForgejoHandlerConfig {
            require_branch: Some("main".to_string()),
            default_phase: "check".to_string(),
        };
        let result = handle_forgejo_payload(&payload, &config);
        match result {
            ForgejoHandlerResult::Skipped { reason } => assert!(reason.contains("branch_mismatch")),
            _ => panic!("expected Skipped"),
        }
    }

    #[test]
    fn handler_triggers_on_main() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/main".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        let config = ForgejoHandlerConfig::default();
        let result = handle_forgejo_payload(&payload, &config);
        match result {
            ForgejoHandlerResult::Triggered { source_commit, .. } => {
                assert_eq!(source_commit, "a".repeat(40));
            }
            _ => panic!("expected Triggered"),
        }
    }
}
#[cfg(test)]
mod webhook_server_tests {
    use super::*;

    fn valid_sha() -> String {
        "a".repeat(40)
    }

    #[test]
    fn validate_source_commit_accepts_valid_sha() {
        assert!(validate_source_commit(&valid_sha()));
    }

    #[test]
    fn validate_source_commit_rejects_short() {
        assert!(!validate_source_commit("abc123"));
    }

    #[test]
    fn validate_source_commit_rejects_non_hex() {
        assert!(!validate_source_commit(&"z".repeat(40)));
    }

    #[test]
    fn validate_source_commit_rejects_empty() {
        assert!(!validate_source_commit(""));
    }

    #[test]
    fn error_result_has_correct_fields() {
        let r = error_result("test error");
        assert_eq!(r.status, "error");
        assert!(!r.build_triggered);
        assert_eq!(r.build_proof, None);
        assert!(!r.signature_verified);
        assert_eq!(r.source_commit, None);
        assert_eq!(r.workload_id, None);
    }

    #[test]
    fn hmac_signature_verifies_and_rejects() {
        let secret = "test-secret";
        let body = br#"{"after":"abc"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let good = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(secret, body, &good));
        assert!(!verify_signature(secret, body, "deadbeef"));
        assert!(!verify_signature(secret, b"other", &good));
        assert!(!verify_signature("wrong", body, &good));
    }

    #[test]
    fn content_length_parses_case_insensitively() {
        assert_eq!(
            content_length(b"POST /x HTTP/1.0\r\nContent-Length: 12"),
            12
        );
        assert_eq!(content_length(b"POST /x HTTP/1.0\r\ncontent-length: 5"), 5);
        assert_eq!(content_length(b"POST /x HTTP/1.0\r\nHost: y"), 0);
    }
}
