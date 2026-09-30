//! Pure secret-use governance: capability-based authorization for secret resolution.
//!
//! An agent NEVER receives secret material directly. It submits a `SecretUseRequest`
//! naming the secret, the purpose, its own identity, and the ROLE it acts under.
//! A pure decision function rules `Allowed` or `Denied` against a policy that is
//! DERIVED from the declarative RBAC in `infra/agent-roles.json`.
//!
//! Authority model — every rule restates a declaration, none is invented:
//! - `roles.<role>.secret_source_class` names the secret CLASS a role may consume.
//!   A role declaring `"None"` consumes nothing.
//! - `domains.secrets.mutation_policy` (`read_only`) gates mutation purposes.
//! - `agent_governance_rule[0]`: "An agent must refuse any action whose capability
//!   is absent from its declared role in infra/agent-roles.json".
//! - `roles.human_admin.cannot[2]`: "never touches RuntimeMachineIdentity secrets
//!   post-bootstrap" — the constituent power is not an agent role.
//!
//! This module is 100% pure — no I/O, no clock, no randomness. Callers pass
//! `now_unix_ms` explicitly so decisions are reproducible and testable.
//!
//! KNOWN UNKNOWN: no declared mapping from `agent_id` to `role` exists today.
//! A claim carries `agent_id` and `session_id`, not a role. The role must come
//! from an admitted claim or an enrollment record, never from the agent's own
//! assertion. Until that binding is declared, the role is a caller-supplied
//! claim that is audited, not authenticated.

use serde::{Deserialize, Serialize};

use crate::secret_reference::{SecretClass, SecretReference};

/// Why an agent wants to use a secret. Purposes are closed and auditable;
/// an unlisted purpose is a denial, never an implicit allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUsePurpose {
    /// Inject into a short-lived child process environment.
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

    /// The purpose set a role may exercise when it declares a secret class:
    /// observe the posture, and consume the value through a bounded injection.
    pub fn consumption_set() -> Vec<Self> {
        vec![Self::ObservePosture, Self::InjectProcess]
    }
}

/// Domain-level mutation policy, as declared in `domains.<name>.mutation_policy`.
/// The `secrets` domain declares `read_only`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretMutationPolicy {
    /// No mutation of stored secret material is permitted.
    ReadOnly,
    /// Mutation requires an explicit `--apply` gesture.
    ExplicitApplyOnly,
    /// Mutation requires an admitted dispatch or explicit apply.
    AdmittedDispatchOrExplicitApplyOnly,
    /// Policy string present but unrecognised. Fail closed: treated as ReadOnly.
    Unrecognised,
}

impl SecretMutationPolicy {
    /// Parse the declarative spelling. Unknown values fail closed to `Unrecognised`,
    /// which denies mutation — never silently permits.
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "read_only" => Self::ReadOnly,
            "explicit_apply_only" => Self::ExplicitApplyOnly,
            "admitted_dispatch_or_explicit_apply_only" => Self::AdmittedDispatchOrExplicitApplyOnly,
            _ => Self::Unrecognised,
        }
    }

    /// Whether this policy permits a purpose that mutates stored material.
    pub fn permits_mutation(self) -> bool {
        matches!(
            self,
            Self::ExplicitApplyOnly | Self::AdmittedDispatchOrExplicitApplyOnly
        )
    }
}

/// Parse a `secret_source_class` declaration into the classes it names.
///
/// Accepts the declarative spellings in `infra/agent-roles.json`: `"None"`,
/// `"HumanAdmin"`, `"MachineEnrollment"`, `"RuntimeMachineIdentity"`,
/// `"RecoveryBreakglass"`, and pipe-separated unions such as
/// `"HumanAdmin | RuntimeMachineIdentity"`.
///
/// Unknown tokens are dropped and never widened into a permission.
pub fn parse_secret_source_class(raw: &str) -> Vec<SecretClass> {
    let mut out = Vec::new();
    for token in raw.split('|') {
        let class = match token.trim() {
            "HumanAdmin" => Some(SecretClass::HumanAdmin),
            "MachineEnrollment" => Some(SecretClass::MachineEnrollment),
            "RuntimeMachineIdentity" => Some(SecretClass::RuntimeMachineIdentity),
            "RecoveryBreakglass" => Some(SecretClass::RecoveryBreakglass),
            _ => None,
        };
        if let Some(class) = class {
            if !out.contains(&class) {
                out.push(class);
            }
        }
    }
    out
}

/// The role name reserved for the constituent power. Agents never act as it.
pub const HUMAN_ADMIN_ROLE: &str = "human_admin";

/// A request from an agent identity to use a secret.
///
/// The agent supplies its identity, the role it acts under, the secret reference,
/// the purpose, and an optional CYNIC claim lease id. It never supplies the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseRequest {
    /// Identity of the requesting agent (e.g. `agent-code-ssh-extractor`).
    pub agent_id: String,
    /// The declared role the agent acts under. Must come from an admitted claim
    /// or enrollment record — never from the agent's own assertion.
    pub role: String,
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
        role: impl Into<String>,
        reference: SecretReference,
        purpose: SecretUsePurpose,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            role: role.into(),
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

/// A declared rule permitting a ROLE to consume a set of secret CLASSES for a
/// set of purposes. Derived from `roles.<role>` in `infra/agent-roles.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseGrant {
    /// The declared role this grant applies to. Exact match — no wildcards,
    /// because authority is a named capability, not a pattern.
    pub role: String,
    /// Secret classes this role may consume. Empty means the role declares no
    /// secret source and may consume nothing.
    pub secret_classes: Vec<SecretClass>,
    /// Purposes this role may exercise. Empty means no purpose is permitted.
    pub purposes: Vec<SecretUsePurpose>,
    /// Whether a CYNIC claim lease id is required for value-exposing purposes.
    pub requires_claim: bool,
}

impl SecretUseGrant {
    pub fn matches_role(&self, role: &str) -> bool {
        self.role == role
    }

    pub fn permits_class(&self, class: SecretClass) -> bool {
        self.secret_classes.contains(&class)
    }

    pub fn permits_purpose(&self, purpose: SecretUsePurpose) -> bool {
        self.purposes.contains(&purpose)
    }
}

/// The declared policy: role grants plus the domain mutation policy.
/// Deny by default — an empty grant list denies everything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUsePolicy {
    pub grants: Vec<SecretUseGrant>,
    pub mutation_policy: SecretMutationPolicy,
}

impl Default for SecretUsePolicy {
    fn default() -> Self {
        Self::deny_all()
    }
}

impl SecretUsePolicy {
    pub fn new(grants: Vec<SecretUseGrant>, mutation_policy: SecretMutationPolicy) -> Self {
        Self {
            grants,
            mutation_policy,
        }
    }

    /// The empty policy denies everything — fail-closed.
    pub fn deny_all() -> Self {
        Self {
            grants: Vec::new(),
            mutation_policy: SecretMutationPolicy::ReadOnly,
        }
    }

    fn grant_for(&self, role: &str) -> Option<&SecretUseGrant> {
        self.grants.iter().find(|g| g.matches_role(role))
    }
}

/// The verdict of a pure policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum SecretUseDecision {
    /// The request is permitted; the gatekeeper may resolve and inject.
    Allowed {
        /// The role that permitted it (for the audit trail).
        matched_role: String,
    },
    /// The request is refused, with a machine-readable reason.
    Denied { reason: SecretUseDenialReason },
}

/// Why a secret-use request was denied. Every variant is auditable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretUseDenialReason {
    /// An agent identity or role was empty — unattributable use is refused.
    UnattributableRequester,
    /// The constituent-power role cannot be assumed by an agent.
    HumanAdminRoleNotAssumable,
    /// The purpose mutates material but the domain policy is read-only.
    MutationPolicyReadOnly,
    /// The role is absent from the declared policy.
    RoleNotDeclared,
    /// The role does not declare this secret class in its `secret_source_class`.
    SecretClassNotDeclaredForRole,
    /// The role does not declare this purpose.
    PurposeNotPermittedForRole,
    /// The grant demands a claim lease the request lacks.
    ClaimLeaseRequired,
}

/// Pure decision function: evaluate a request against a derived policy.
///
/// Rules, in order. Every rule restates a declaration; none is invented:
/// 1. Empty `agent_id` or `role` is denied — unattributable use is never allowed.
/// 2. The `human_admin` role is denied — the constituent power is not an agent.
/// 3. A mutating purpose under a read-only policy is denied.
/// 4. A role with no grant is denied.
/// 5. A class the role does not declare is denied.
/// 6. A purpose the role does not declare is denied.
/// 7. A claim-requiring value-exposing purpose without a claim is denied.
/// 8. Otherwise allowed.
///
/// Pure: no I/O, no clock. `now_unix_ms` is accepted for audit correlation
/// but does not influence the verdict today — reserved for time-bounded grants.
pub fn decide_secret_use(
    request: &SecretUseRequest,
    policy: &SecretUsePolicy,
    _now_unix_ms: u64,
) -> SecretUseDecision {
    if request.agent_id.trim().is_empty() || request.role.trim().is_empty() {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::UnattributableRequester,
        };
    }

    if request.role == HUMAN_ADMIN_ROLE {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::HumanAdminRoleNotAssumable,
        };
    }

    if request.purpose.mutates_material() && !policy.mutation_policy.permits_mutation() {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::MutationPolicyReadOnly,
        };
    }

    let Some(grant) = policy.grant_for(&request.role) else {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::RoleNotDeclared,
        };
    };

    if !grant.permits_class(request.reference.class) {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::SecretClassNotDeclaredForRole,
        };
    }

    if !grant.permits_purpose(request.purpose) {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::PurposeNotPermittedForRole,
        };
    }

    let has_claim = request
        .claim_lease_id
        .as_ref()
        .is_some_and(|c| !c.trim().is_empty());

    if request.purpose.exposes_value() && grant.requires_claim && !has_claim {
        return SecretUseDecision::Denied {
            reason: SecretUseDenialReason::ClaimLeaseRequired,
        };
    }

    SecretUseDecision::Allowed {
        matched_role: grant.role.clone(),
    }
}

/// One append-only audit entry recording a secret-use attempt.
/// NEVER contains the secret value, its length, or its shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretUseAuditEntry {
    pub schema: &'static str,
    pub at_unix_ms: u64,
    pub agent_id: String,
    pub role: String,
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
            role: request.role.clone(),
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

    fn ref_of(id: &str, class: SecretClass) -> SecretReference {
        SecretReference {
            id: id.to_string(),
            class,
            provider: SecretProvider::PrincipalOwnedFile,
            locator: id.to_string(),
        }
    }

    fn runtime_ref(id: &str) -> SecretReference {
        ref_of(id, SecretClass::RuntimeMachineIdentity)
    }

    /// Derive the policy the way the loader does from `infra/agent-roles.json`:
    /// one grant per role, classes from `secret_source_class`, purposes from the
    /// consumption set, claim required for value exposure.
    fn derived_policy() -> SecretUsePolicy {
        let mut grants = Vec::new();
        for (role, source_class) in [
            ("human_admin", "HumanAdmin"),
            ("build_runner", "MachineEnrollment"),
            ("fleet_coordinator", "RuntimeMachineIdentity"),
            ("scheduler_agent", "RuntimeMachineIdentity"),
            ("runtime_publisher", "RuntimeMachineIdentity"),
            ("schema_warden", "None"),
            ("surface_inspector", "None"),
            ("metabolic_auditor", "None"),
        ] {
            grants.push(SecretUseGrant {
                role: role.to_string(),
                secret_classes: parse_secret_source_class(source_class),
                purposes: SecretUsePurpose::consumption_set(),
                requires_claim: true,
            });
        }
        SecretUsePolicy::new(grants, SecretMutationPolicy::ReadOnly)
    }

    fn request(role: &str, class: SecretClass) -> SecretUseRequest {
        SecretUseRequest::new(
            "agent-code",
            role,
            ref_of("proxmox-token", class),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_test_1")
    }

    #[test]
    fn empty_agent_or_role_is_denied() {
        let policy = derived_policy();
        let no_agent = SecretUseRequest::new(
            "",
            "scheduler_agent",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        let no_role = SecretUseRequest::new(
            "agent-code",
            "",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        for req in [no_agent, no_role] {
            assert_eq!(
                decide_secret_use(&req, &policy, 0),
                SecretUseDecision::Denied {
                    reason: SecretUseDenialReason::UnattributableRequester
                }
            );
        }
    }

    #[test]
    fn human_admin_role_is_never_assumable_by_an_agent() {
        let policy = derived_policy();
        let req = request(HUMAN_ADMIN_ROLE, SecretClass::HumanAdmin);
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::HumanAdminRoleNotAssumable
            }
        );
    }

    #[test]
    fn role_may_consume_only_its_declared_class() {
        let policy = derived_policy();
        // scheduler_agent declares RuntimeMachineIdentity only.
        let allowed = request("scheduler_agent", SecretClass::RuntimeMachineIdentity);
        assert!(matches!(
            decide_secret_use(&allowed, &policy, 0),
            SecretUseDecision::Allowed { .. }
        ));

        let wrong_class = request("scheduler_agent", SecretClass::MachineEnrollment);
        assert_eq!(
            decide_secret_use(&wrong_class, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::SecretClassNotDeclaredForRole
            }
        );
    }

    #[test]
    fn role_declaring_none_consumes_nothing() {
        let policy = derived_policy();
        let req = request("schema_warden", SecretClass::RuntimeMachineIdentity);
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::SecretClassNotDeclaredForRole
            }
        );
    }

    #[test]
    fn undeclared_role_is_denied() {
        let policy = derived_policy();
        let req = request("agent-rogue", SecretClass::RuntimeMachineIdentity);
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::RoleNotDeclared
            }
        );
    }

    #[test]
    fn read_only_policy_denies_mutation() {
        let policy = derived_policy();
        for purpose in [
            SecretUsePurpose::Rotate,
            SecretUsePurpose::Install,
            SecretUsePurpose::Delete,
        ] {
            let req = SecretUseRequest::new(
                "agent-code",
                "scheduler_agent",
                runtime_ref("proxmox-token"),
                purpose,
            )
            .with_claim("claim_1");
            assert_eq!(
                decide_secret_use(&req, &policy, 0),
                SecretUseDecision::Denied {
                    reason: SecretUseDenialReason::MutationPolicyReadOnly
                },
                "purpose {purpose:?} must be denied under read_only"
            );
        }
    }

    #[test]
    fn observe_posture_never_requires_a_claim() {
        let policy = derived_policy();
        let req = SecretUseRequest::new(
            "agent-code",
            "scheduler_agent",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::ObservePosture,
        );
        assert!(matches!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Allowed { .. }
        ));
    }

    #[test]
    fn value_exposure_requires_a_claim() {
        let policy = derived_policy();
        let no_claim = SecretUseRequest::new(
            "agent-code",
            "scheduler_agent",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        assert_eq!(
            decide_secret_use(&no_claim, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::ClaimLeaseRequired
            }
        );

        let with_claim = no_claim.with_claim("claim_agentcodesshextractor_1790778389");
        assert_eq!(
            decide_secret_use(&with_claim, &policy, 0),
            SecretUseDecision::Allowed {
                matched_role: "scheduler_agent".to_string()
            }
        );
    }

    #[test]
    fn undeclared_purpose_is_denied() {
        let policy = SecretUsePolicy::new(
            vec![SecretUseGrant {
                role: "auditor".to_string(),
                secret_classes: vec![SecretClass::RuntimeMachineIdentity],
                purposes: vec![SecretUsePurpose::ObservePosture],
                requires_claim: false,
            }],
            SecretMutationPolicy::ReadOnly,
        );
        let req = SecretUseRequest::new(
            "agent-code",
            "auditor",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        assert_eq!(
            decide_secret_use(&req, &policy, 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::PurposeNotPermittedForRole
            }
        );
    }

    #[test]
    fn deny_all_policy_refuses_everything() {
        let req = request("scheduler_agent", SecretClass::RuntimeMachineIdentity);
        assert_eq!(
            decide_secret_use(&req, &SecretUsePolicy::deny_all(), 0),
            SecretUseDecision::Denied {
                reason: SecretUseDenialReason::RoleNotDeclared
            }
        );
    }

    #[test]
    fn source_class_parsing_is_declarative() {
        assert_eq!(parse_secret_source_class("None"), Vec::new());
        assert_eq!(
            parse_secret_source_class("HumanAdmin"),
            vec![SecretClass::HumanAdmin]
        );
        assert_eq!(
            parse_secret_source_class("HumanAdmin | RuntimeMachineIdentity"),
            vec![SecretClass::HumanAdmin, SecretClass::RuntimeMachineIdentity]
        );
        // Unknown tokens never widen into a permission.
        assert_eq!(
            parse_secret_source_class("Bogus | HumanAdmin"),
            vec![SecretClass::HumanAdmin]
        );
        assert_eq!(parse_secret_source_class("Bogus"), Vec::new());
    }

    #[test]
    fn mutation_policy_parsing_fails_closed() {
        assert_eq!(
            SecretMutationPolicy::parse("read_only"),
            SecretMutationPolicy::ReadOnly
        );
        assert!(!SecretMutationPolicy::parse("read_only").permits_mutation());
        assert!(!SecretMutationPolicy::parse("something_unknown").permits_mutation());
        assert!(SecretMutationPolicy::parse("explicit_apply_only").permits_mutation());
    }

    #[test]
    fn audit_entry_carries_no_secret_material_and_records_role() {
        let req = request("scheduler_agent", SecretClass::RuntimeMachineIdentity)
            .with_justification("provision ephemeral LXC");
        let decision = SecretUseDecision::Allowed {
            matched_role: "scheduler_agent".to_string(),
        };
        let entry = SecretUseAuditEntry::from_decision(&req, decision, 1790778389000);
        let json = serde_json::to_string(&entry).expect("serialize audit entry");

        assert!(json.contains("cynic.secret_use_audit.v1"));
        assert!(json.contains("proxmox-token"));
        assert!(json.contains("scheduler_agent"));
        assert!(!json.contains("value"));
        assert!(!json.contains("length"));
        assert!(!json.contains("shape"));
        assert!(!json.contains("commitment"));
    }

    #[test]
    fn decision_is_deterministic() {
        let policy = derived_policy();
        let req = request("scheduler_agent", SecretClass::RuntimeMachineIdentity);
        let a = decide_secret_use(&req, &policy, 111);
        let b = decide_secret_use(&req, &policy, 999);
        assert_eq!(a, b, "verdict must not depend on the clock today");
    }
}
