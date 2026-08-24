//! Forgejo personal access token lifecycle types.
//!
//! These types describe the desired state, observation, and proof for a
//! Forgejo API token that the runtime manages without requiring a human
//! admin secret. The token is created by delegating through an injected
//! execution port (SSH-local Exec or HTTP API), so the domain layer never
//! hard-wires `Command::new("forgejo")`.

use serde::{Deserialize, Serialize};

pub const FORGEJO_TOKEN_OBSERVATION_SCHEMA: &str = "cynic.forgejo_token_observation.v1";
pub const FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA: &str = "cynic.forgejo_token_provision_report.v1";

/// Where the runtime writes the provisioned Forgejo token.
///
/// Matches the path already consulted by `RuntimeForgejoObserver`
/// in `cynic-host/src/forgejo_control.rs`:
/// `/etc/cynic/secrets/forgejo-token`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenSpec {
    pub schema: String,
    /// The Forgejo principal that owns the token (e.g. "cynic-cortex").
    pub principal: String,
    /// Scopes granted to the new token (Forgejo scope syntax).
    pub scopes: Vec<String>,
    /// Where to write the token material on the local filesystem.
    pub target_path: String,
    /// Unix mode for the installed secret file (always 0o640 in practice).
    pub mode: u32,
    /// Expiry hint in seconds (optional; Forgejo may ignore depending on config).
    pub expiry_seconds: Option<u64>,
}

impl Default for ForgejoTokenSpec {
    fn default() -> Self {
        Self {
            schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
            principal: "cynic-cortex".to_string(),
            scopes: vec![
                "read:organization".to_string(),
                "write:organization".to_string(),
                "write:repository".to_string(),
            ],
            target_path: "/etc/cynic/secrets/forgejo-token".to_string(),
            mode: 0o640,
            expiry_seconds: None,
        }
    }
}

/// Read-only account of the Forgejo token secret on disk and its validity
/// against the Forgejo instance.
///
/// `valid` is the only authoritative field that can be `Proven` here; the
/// raw token bytes are never embedded in an observation (see
/// `runtime_secret_install.rs` for the parallel SecretPresence contract).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenObservation {
    pub schema: String,
    pub principal: String,
    pub path: String,
    /// File-level presence / ownership / mode (never contains the token value).
    pub secret_present: bool,
    pub owner_root: Option<bool>,
    pub mode_o640: Option<bool>,
    /// Whether the token passes a live `forgejo api /api/v1/user` check
    /// through the chosen transport.
    pub api_authenticated: ForgejoTokenAuthState,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
}

/// Auth state mirrors `ForgejoEvidenceState` semantics but is scoped to token
/// validity so a single concern owns the meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoTokenAuthState {
    /// Token is present on disk AND the live API call succeeded.
    Proven,
    /// Token is present but the API call failed (bad/expired/revoked).
    Refuted,
    /// No token on disk.
    Absent,
    /// Token file present but we could not run a live probe.
    Unknown,
}

/// Proof emitted after `apply`. Never contains the raw token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenProvisionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub principal: String,
    pub mutated: bool,
    /// Before/after read-back (the observation, not the secret).
    pub before: ForgejoTokenObservation,
    pub after: ForgejoTokenObservation,
    /// Non-empty iff status == "completed".
    pub proofs: Vec<String>,
    /// Token metadata returned by Forgejo (id, name, sha1). No secret value.
    pub token_metadata: Option<ForgejoTokenMetadata>,
    pub rollback_intent: String,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenMetadata {
    pub token_id: u64,
    pub name: String,
    pub sha1: String,
}

impl std::default::Default for ForgejoTokenMetadata {
    fn default() -> Self {
        Self {
            token_id: 0,
            name: String::new(),
            sha1: String::new(),
        }
    }
}

/// Reconcile decision: produces a pure plan from desired + observed state.
/// This lives in the protocol crate so CLI/MCP surfaces project from one
/// canonical decision rather than re-deriving it locally.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlan {
    pub schema: String,
    pub status: String,
    pub desired: ForgejoTokenSpec,
    pub actions: Vec<ForgejoTokenPlannedAction>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgejoTokenPlannedAction {
    pub kind: ForgejoTokenActionKind,
    pub mutates: bool,
    /// True when the action can be performed by a runtime SSH/exec adapter
    /// alone (no human admin login required on Forgejo web UI).
    pub requires_human_admin_secret: bool,
    pub proof: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgejoTokenActionKind {
    /// Probe the token file presence and API validity.
    ObserveToken,
    /// Create a new token through the injected port (SSH-local Exec or HTTP).
    CreateToken,
    /// Remove a token whose `sha1` matches the expected value (rotation).
    RotateToken,
}

pub fn plan_forgejo_token_convergence(
    desired: ForgejoTokenSpec,
    observed: &ForgejoTokenObservation,
) -> ForgejoTokenPlan {
    let mut actions = Vec::new();
    let mut blockers = Vec::new();

    // Always observe first so the plan is grounded in a fresh read.
    actions.push(ForgejoTokenPlannedAction {
        kind: ForgejoTokenActionKind::ObserveToken,
        mutates: false,
        requires_human_admin_secret: false,
        proof: "cynic.forgejo_token_observation.v1".to_string(),
    });

    match observed.api_authenticated {
        ForgejoTokenAuthState::Proven => {
            // valid token — nothing to mutate
        }
        ForgejoTokenAuthState::Refuted => {
            // present but bad → rotate via the port (SSH-local Exec)
            blockers.push("forgejo_token_present_but_api_call_refuted".to_string());
            actions.push(ForgejoTokenPlannedAction {
                kind: ForgejoTokenActionKind::CreateToken,
                mutates: true,
                requires_human_admin_secret: false,
                proof: "forgejo_token_provisioned_via_runtime_adapter".to_string(),
            });
        }
        ForgejoTokenAuthState::Absent => {
            actions.push(ForgejoTokenPlannedAction {
                kind: ForgejoTokenActionKind::CreateToken,
                mutates: true,
                requires_human_admin_secret: false,
                proof: "forgejo_token_provisioned_via_runtime_adapter".to_string(),
            });
        }
        ForgejoTokenAuthState::Unknown => {
            blockers.push("forgejo_token_observation_unknown".to_string());
        }
    }

    ForgejoTokenPlan {
        schema: "cynic.forgejo_token_plan.v1".to_string(),
        status: if blockers.is_empty() {
            "ready".to_string()
        } else {
            "blocked".to_string()
        },
        desired,
        actions,
        blockers,
        evidence_refs: observed.evidence_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(auth: ForgejoTokenAuthState, present: bool) -> ForgejoTokenObservation {
        ForgejoTokenObservation {
            schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
            principal: "cynic-cortex".to_string(),
            path: "/etc/cynic/secrets/forgejo-token".to_string(),
            secret_present: present,
            owner_root: Some(true),
            mode_o640: Some(true),
            api_authenticated: auth,
            unknowns: Vec::new(),
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn absent_token_plans_creation() {
        let plan = plan_forgejo_token_convergence(
            ForgejoTokenSpec::default(),
            &observed(ForgejoTokenAuthState::Absent, false),
        );
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.actions[0].kind, ForgejoTokenActionKind::ObserveToken);
        assert_eq!(plan.actions[1].kind, ForgejoTokenActionKind::CreateToken);
        assert!(!plan.actions[1].requires_human_admin_secret);
    }

    #[test]
    fn refuted_token_plans_rotation() {
        let plan = plan_forgejo_token_convergence(
            ForgejoTokenSpec::default(),
            &observed(ForgejoTokenAuthState::Refuted, true),
        );
        assert!(
            plan.blockers
                .contains(&"forgejo_token_present_but_api_call_refuted".to_string())
        );
        assert_eq!(plan.actions.len(), 2);
        assert_eq!(plan.actions[1].kind, ForgejoTokenActionKind::CreateToken);
    }

    #[test]
    fn proven_token_is_noop() {
        let plan = plan_forgejo_token_convergence(
            ForgejoTokenSpec::default(),
            &observed(ForgejoTokenAuthState::Proven, true),
        );
        assert_eq!(plan.status, "ready");
        assert_eq!(plan.actions.len(), 1);
        assert!(plan.blockers.is_empty());
    }

    #[test]
    fn unknown_token_blocks() {
        let plan = plan_forgejo_token_convergence(
            ForgejoTokenSpec::default(),
            &observed(ForgejoTokenAuthState::Unknown, true),
        );
        assert_eq!(plan.status, "blocked");
        assert!(plan.actions.iter().all(|a| !a.mutates));
    }
}
