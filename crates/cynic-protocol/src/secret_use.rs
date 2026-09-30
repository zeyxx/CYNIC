//! Pure secret-use governance: capability-based authorization for secret resolution.
//!
//! An agent NEVER receives secret material directly. It receives a `SecretUseRequest`
//! naming the secret, the purpose, and its own agent identity. A pure decision
//! function rules `Allowed` or `Denied` against a declared policy. The gatekeeper
//! (host layer) is the only code allowed to call `resolve()` after an `Allowed`
//! verdict, and it records an append-only audit entry — never the value.
//!
//! Authority: AGENTS.md secret classes table; `contracts/foundation.json`
//! (HumanAdmin = bootstrap only, RuntimeMachineIdentity = steady state).
//!
//! This module is 100% pure — no I/O, no clock, no randomness. Callers pass
//! `now_unix_ms` explicitly so decisions are reproducible and testable.

use serde::{Deserialize, Serialize};

use crate::secret_reference::{SecretClass, SecretReference};

/// Why an agent wants to use a secret. Purposes are closed and auditable;
/// an unlisted purpose is a denial, never an implicit allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUsePurpose {
    /// Inject into a short-lived child process argv/environment.
    InjectProcess,
    /// Read presence/posture metadata only — never the value.
    ObservePosture,
    /// Rotate the secret to a new value.
    Rotate,
    /// Install a new secret value from an external source.
    Install,
    /// Delete the secret.
    Delete,
}

impl SecretUsePurpose {
    /// Purposes that expose the plaintext value to a process.
    pub fn exposes_value(self) -> bool {
        matches!(self, Self::InjectProcess)
    }

    /// Purposes that mutate stored material.
    pub fn mutates_material(self) -> bool {
        matches!(self, Self::Rotate | Self::Install | Self::Delete)
    }
}

/// A request from an agent identity to use a secret.
///
/// The agent supplies its identity, the secret reference, the purpose, and an
/// optional CYNIC claim lease id. It never supplies the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseRequest {
    /// Identity of the requesting agent (e.g. `agent-code-ssh-extractor`).
    pub agent_id: String,
    /// The secret being requested.
    pub reference: SecretReference,
    /// What the agent intends to do with it.
    pub purpose: SecretUsePurpose,
    /// Optional CYNIC claim lease id proving the agent holds the scope.
    pub claim_lease_id: Option<String>,
    /// Optional short human/agent-readable justification, for the audit trail.
    pub justification: Option<String>,
}

impl SecretUseRequest {
    pub fn new(
        agent_id: impl Into<String>,
        reference: SecretReference,
        purpose: SecretUsePurpose,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            reference,
            purpose,
            claim_lease_id: None,
            justification: None,
        }
    }

    pub fn with_claim(mut self, lease_id: impl Into<String>) -> Self {
        self.claim_lease_id = Some(lease_id.into());
        self
    }

    pub fn with_justification(mut self, text: impl Into<String>) -> Self {
        self.justification = Some(text.into());
        self
    }
}

/// A declared rule permitting an agent identity to use a secret class/purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseGrant {
    /// Agent identity pattern. `*` matches any agent; otherwise exact match.
    pub agent_pattern: String,
    /// Secret ids this grant applies to. Empty = all ids.
    pub secret_ids: Vec<String>,
    /// Purposes allowed by this grant. Empty = all purposes.
    pub purposes: Vec<SecretUsePurpose>,
    /// Whether the requester must present a CYNIC claim lease id.
    pub requires_claim: bool,
}

impl SecretUseGrant {
    pub fn matches_agent(&self, agent_id: &str) -> bool {
        self.agent_pattern == "*" || self.agent_pattern == agent_id
    }

    pub fn matches_secret(&self, secret_id: &str) -> bool {
        self.secret_ids.is_empty() || self.secret_ids.iter().any(|s| s == secret_id)
    }

    pub fn matches_purpose(&self, purpose: SecretUsePurpose) -> bool {
        self.purposes.is_empty() || self.purposes.contains(&purpose)
    }
}

/// The declared policy: an ordered list of grants. Deny by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SecretUsePolicy {
    pub grants: Vec<SecretUseGrant>,
}

impl SecretUsePolicy {
    pub fn new(grants: Vec<SecretUseGrant>) -> Self {
        Self { grants }
    }

    /// The empty policy denies everything — fail-closed.
    pub fn deny_all() -> Self {
        Self { grants: Vec::new() }
    }

    /// The default runtime policy: agents may only observe posture.
    /// Value-exposing and mutating purposes require an explicit grant.
    pub fn observe_only() -> Self {
        Self {
            grants: vec![SecretUseGrant {
                agent_pattern: "*".to_string(),
                secret_ids: Vec::new(),
                purposes: vec![SecretUsePurpose::ObservePosture],
                requires_claim: false,
            }],
        }
    }

    fn matching_grants(&self, request: &SecretUseRequest) -> Vec<&SecretUseGrant> {
        self.grants
            .iter()
            .filter(|g| {
                g.matches_agent(&request.agent_id)
                    && g.matches_secret(&request.reference.id)
                    && g.matches_purpose(request.purpose)
            })
            .collect()
    }
}

/// The verdict of a pure policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum SecretUseDecision {
    /// The request is permitted; the gatekeeper may resolve and inject.
    Allowed {
        /// The grant that permitted it (for the audit trail).
        matched_agent_pattern: String,
    },
    /// The request is refused, with a machine-readable reason.
    Denied { reason: SecretUseDenialReason },
}

/// Why a secret-use request was denied. Every variant is auditable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUseDenialReason {
    /// No grant matched agent + secret + purpose.
    NoMatchingGrant,
    /// A grant matched but demands a claim lease the request lacks.
    ClaimLeaseRequired,
    /// `HumanAdmin` secrets cannot be used by agents at all.
    HumanAdminNotDelegable,
    /// An agent identity was empty — unattributable use is refused.
    UnattributableRequester,
}

/// Pure decision function: evaluate a request against a policy.
///
/// Rules, in order:
/// 1. An empty `agent_id` is denied — unattributable use is never allowed.
/// 2. `HumanAdmin` secrets are denied for agent use regardless of grants.
/// 3. A matching grant that `requires_claim` needs a non-empty `claim_lease_id`.
/// 4. Any matching grant that does not require a claim permits the request.
/// 5. No matching grant denies.
///
/// Pure: no I/O, no clock. `now_unix_ms` is accepted for audit correlation
/// but does not influence the verdict today — reserved for time-bounded grants.
pub fn decide_secret_use(
    request: &SecretUseRequest,
    policy: &SecretUsePolicy,
    _now_unix_ms: u64,
) -> SecretUseDecision {
    if request.agent_id.trim().is_empty() {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::UnattributableRequester,
        };
    }

    if request.reference.class == SecretClass::HumanAdmin {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::HumanAdminNotDelegable,
        };
    }

    let matching = policy.matching_grants(request);
    if matching.is_empty() {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::NoMatchingGrant,
        };
    }

    let has_claim = request
        .claim_lease_id
        .as_ref()
        .is_some_and(|c| !c.trim().is_empty());

    // A claim-satisfying grant wins; a claim-free grant permits outright.
    let mut claim_blocked = false;
    for grant in &matching {
        if grant.requires_claim && !has_claim {
            claim_blocked = true;
            continue;
        }
        return SecretUseDecision::Allowed {
            matched_agent_pattern: grant.agent_pattern.clone(),
        };
    }

    if claim_blocked {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::ClaimLeaseRequired,
        };
    }

    SecretUseDecision::Denied {
        reason: SecretUseDenialReason::NoMatchingGrant,
    }
}

/// One append-only audit entry recording a secret-use attempt.
/// NEVER contains the secret value, its length, or its shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseAuditEntry {
    pub schema: &'static str,
    pub at_unix_ms: u64,
    pub agent_id: String,
    pub secret_id: String,
    pub secret_class: SecretClass,
    pub purpose: SecretUsePurpose,
    pub claim_lease_id: Option<String>,
    pub decision: SecretUseDecision,
    pub justification: Option<String>,
}

pub const SECRET_USE_AUDIT_SCHEMA: &str = "cynic.secret_use_audit.v1";

impl SecretUseAuditEntry {
    /// Build an audit entry from a request, its decision, and a timestamp.
    /// The value is structurally absent — this type cannot carry it.
    pub fn from_decision(
        request: &SecretUseRequest,
        decision: SecretUseDecision,
        at_unix_ms: u64,
    ) -> Self {
        Self {
            schema: SECRET_USE_AUDIT_SCHEMA,
            at_unix_ms,
            agent_id: request.agent_id.clone(),
            secret_id: request.reference.id.clone(),
            secret_class: request.reference.class,
            purpose: request.purpose,
            claim_lease_id: request.claim_lease_id.clone(),
            decision,
            justification: request.justification.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secret_reference::SecretProvider;

    fn ref_runtime(id: &str) -> SecretReference {
        SecretReference {
            id: id.to_string(),
            class: SecretClass::RuntimeMachineIdentity,
            provider: SecretProvider::PrincipalOwnedFile,
            locator: id.to_string(),
        }
    }

    fn ref_human(id: &str) -> SecretReference {
        SecretReference {
            id: id.to_string(),
            class: SecretClass::HumanAdmin,
            provider: SecretProvider::PrincipalOwnedFile,
            locator: id.to_string(),
        }
    }

    fn inject_grant(agent: &str, ids: &[&str]) -> SecretUseGrant {
        SecretUseGrant {
            agent_pattern: agent.to_string(),
            secret_ids: ids.iter().map(|s| s.to_string()).collect(),
            purposes: vec![SecretUsePurpose::InjectProcess],
            requires_claim: true,
        }
    }

    #[test]
    fn empty_agent_is_denied() {
        let req = SecretUseRequest::new(
            "",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        let policy = SecretUsePolicy::new(vec![inject_grant("*", &[])]);
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::UnattributableRequester
            }
        );
    }

    #[test]
    fn human_admin_is_never_delegable() {
        let req = SecretUseRequest::new(
            "agent-x",
            ref_human("root-password"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        let policy = SecretUsePolicy::new(vec![SecretUseGrant {
            agent_pattern: "*".to_string(),
            secret_ids: Vec::new(),
            purposes: Vec::new(),
            requires_claim: false,
        }]);
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::HumanAdminNotDelegable
            }
        );
    }

    #[test]
    fn deny_all_policy_refuses_everything() {
        let req = SecretUseRequest::new(
            "agent-x",
            ref_runtime("forgejo-token"),
            SecretUsePurpose::ObservePosture,
        );
        assert_eq!(
            decide_secret_use(&req, &SecretUsePolicy::deny_all(), 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::NoMatchingGrant
            }
        );
    }

    #[test]
    fn observe_only_permits_posture_and_refuses_injection() {
        let policy = SecretUsePolicy::observe_only();
        let observe = SecretUseRequest::new(
            "agent-x",
            ref_runtime("forgejo-token"),
            SecretUsePurpose::ObservePosture,
        );
        let inject = SecretUseRequest::new(
            "agent-x",
            ref_runtime("forgejo-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");

        assert!(matches!(
            decide_secret_use(&observe, &policy, 0),
            SecretUseDecision::Allowed { .. }
        ));
        assert_eq!(
            decide_secret_use(&inject, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::NoMatchingGrant
            }
        );
    }

    #[test]
    fn claim_requiring_grant_denies_without_claim() {
        let policy = SecretUsePolicy::new(vec![inject_grant("agent-code", &["proxmox-token"])]);
        let no_claim = SecretUseRequest::new(
            "agent-code",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        assert_eq!(
            decide_secret_use(&no_claim, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::ClaimLeaseRequired
            }
        );
    }

    #[test]
    fn claim_requiring_grant_permits_with_claim() {
        let policy = SecretUsePolicy::new(vec![inject_grant("agent-code", &["proxmox-token"])]);
        let with_claim = SecretUseRequest::new(
            "agent-code",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_agentcodesshextractor_1790778389");
        assert_eq!(
            decide_secret_use(&with_claim, &policy, 0),
            SecretUseDecision::Allowed {
                matched_agent_pattern: "agent-code".to_string()
            }
        );
    }

    #[test]
    fn wrong_agent_is_denied() {
        let policy = SecretUsePolicy::new(vec![inject_grant("agent-code", &["proxmox-token"])]);
        let other = SecretUseRequest::new(
            "agent-rogue",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        assert_eq!(
            decide_secret_use(&other, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::NoMatchingGrant
            }
        );
    }

    #[test]
    fn wrong_secret_is_denied() {
        let policy = SecretUsePolicy::new(vec![inject_grant("agent-code", &["proxmox-token"])]);
        let other = SecretUseRequest::new(
            "agent-code",
            ref_runtime("cloudflare-api-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        assert_eq!(
            decide_secret_use(&other, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::NoMatchingGrant
            }
        );
    }

    #[test]
    fn mutation_purposes_are_classified() {
        assert!(SecretUsePurpose::InjectProcess.exposes_value());
        assert!(!SecretUsePurpose::ObservePosture.exposes_value());
        assert!(SecretUsePurpose::Rotate.mutates_material());
        assert!(SecretUsePurpose::Delete.mutates_material());
        assert!(!SecretUsePurpose::ObservePosture.mutates_material());
    }

    #[test]
    fn audit_entry_carries_no_secret_material() {
        let req = SecretUseRequest::new(
            "agent-code",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1")
        .with_justification("provision ephemeral LXC");
        let decision = SecretUseDecision::Allowed {
            matched_agent_pattern: "agent-code".to_string(),
        };
        let entry = SecretUseAuditEntry::from_decision(&req, decision, 1790778389000);
        let json = serde_json::to_string(&entry).expect("serialize audit entry");

        assert!(json.contains("cynic.secret_use_audit.v1"));
        assert!(json.contains("proxmox-token"));
        assert!(!json.contains("value"));
        assert!(!json.contains("length"));
        assert!(!json.contains("shape"));
        assert!(!json.contains("commitment"));
    }

    #[test]
    fn decision_is_deterministic() {
        let policy = SecretUsePolicy::new(vec![inject_grant("agent-code", &["proxmox-token"])]);
        let req = SecretUseRequest::new(
            "agent-code",
            ref_runtime("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        let a = decide_secret_use(&req, &policy, 111);
        let b = decide_secret_use(&req, &policy, 999);
        assert_eq!(a, b, "verdict must not depend on the clock today");
    }
}
