use serde::{Deserialize, Serialize};

/// Stable, machine-readable operator error catalogue.
///
/// A code identifies the failed contract; reports still carry contextual
/// evidence and lower-level blockers separately. It deliberately contains no
/// secrets, paths, or provider-specific command output.
pub const ERROR_CATALOG_SCHEMA: &str = "cynic.error_catalog.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CynicErrorCode {
    BuildAdmissionAuthorityMaterialMissing,
    BuildRunnerUnreachable,
    BuildSchedulerInactive,
    BuildWorktreeDirty,
    BuildAdmissionVerifierUnavailable,
    BuildSourceCommitUnobservable,
    BuildNodeIdentityUnobservable,
    NetworkMeshTransportAbsent,
    NetworkBuildRunnerUnreachable,
    NetworkActuatorMissing,
    LxcMutationAuthorityUnverified,
    BreakglassAdmissionMissing,
    BreakglassAdmissionInvalid,
    BreakglassAdmissionExpired,
    BreakglassActionForbidden,
    BreakglassReceiptConsumed,
    BreakglassAuthorityVerifierUnavailable,
    BreakglassSignatureInvalid,
    BreakglassConsumptionPersistFailed,
    ActivationEpochUnobservable,
    ActivationBootstrapIncomplete,
    ActivationTakeoverProcessInactive,
    ActivationTakeoverIdentityUnverified,
    ActivationReadBackAbsent,
    ActivationConsentMissing,
    ActivationPhaseAuthorityInsufficient,
    AgentScopeConflict,
    AgentClaimExpired,
    AgentSharedTypeDrift,
    AgentMutationWithoutClaim,
    AgentCoordinationStoreUnavailable,
    AgentMergeQueueRace,
    /// An agent requested more budget than its citizenship record grants.
    ///
    /// Budgets are monotonic: a citizen may never amplify its own authority.
    AgentBudgetExceeded,
    FleetObservationBlackout,
}

pub const CYNIC_ERROR_CODES: &[CynicErrorCode] = &[
    CynicErrorCode::BuildAdmissionAuthorityMaterialMissing,
    CynicErrorCode::BuildRunnerUnreachable,
    CynicErrorCode::BuildSchedulerInactive,
    CynicErrorCode::BuildWorktreeDirty,
    CynicErrorCode::BuildAdmissionVerifierUnavailable,
    CynicErrorCode::BuildSourceCommitUnobservable,
    CynicErrorCode::BuildNodeIdentityUnobservable,
    CynicErrorCode::NetworkMeshTransportAbsent,
    CynicErrorCode::NetworkBuildRunnerUnreachable,
    CynicErrorCode::NetworkActuatorMissing,
    CynicErrorCode::LxcMutationAuthorityUnverified,
    CynicErrorCode::BreakglassAdmissionMissing,
    CynicErrorCode::BreakglassAdmissionInvalid,
    CynicErrorCode::BreakglassAdmissionExpired,
    CynicErrorCode::BreakglassActionForbidden,
    CynicErrorCode::BreakglassReceiptConsumed,
    CynicErrorCode::BreakglassAuthorityVerifierUnavailable,
    CynicErrorCode::BreakglassSignatureInvalid,
    CynicErrorCode::BreakglassConsumptionPersistFailed,
    CynicErrorCode::ActivationEpochUnobservable,
    CynicErrorCode::ActivationBootstrapIncomplete,
    CynicErrorCode::ActivationTakeoverProcessInactive,
    CynicErrorCode::ActivationTakeoverIdentityUnverified,
    CynicErrorCode::ActivationReadBackAbsent,
    CynicErrorCode::ActivationConsentMissing,
    CynicErrorCode::ActivationPhaseAuthorityInsufficient,
    CynicErrorCode::AgentScopeConflict,
    CynicErrorCode::AgentClaimExpired,
    CynicErrorCode::AgentSharedTypeDrift,
    CynicErrorCode::AgentMutationWithoutClaim,
    CynicErrorCode::AgentCoordinationStoreUnavailable,
    CynicErrorCode::AgentMergeQueueRace,
    CynicErrorCode::AgentBudgetExceeded,
    CynicErrorCode::FleetObservationBlackout,
];

impl CynicErrorCode {
    pub const fn code(self) -> &'static str {
        match self {
            Self::BuildAdmissionAuthorityMaterialMissing => "CYNIC-BLD-001",
            Self::BuildRunnerUnreachable => "CYNIC-BLD-002",
            Self::BuildSchedulerInactive => "CYNIC-BLD-003",
            Self::BuildWorktreeDirty => "CYNIC-BLD-004",
            Self::BuildAdmissionVerifierUnavailable => "CYNIC-BLD-005",
            Self::BuildSourceCommitUnobservable => "CYNIC-BLD-006",
            Self::BuildNodeIdentityUnobservable => "CYNIC-BLD-007",
            Self::NetworkMeshTransportAbsent => "CYNIC-NET-001",
            Self::NetworkBuildRunnerUnreachable => "CYNIC-NET-002",
            Self::NetworkActuatorMissing => "CYNIC-NET-003",
            Self::LxcMutationAuthorityUnverified => "CYNIC-LXC-001",
            Self::BreakglassAdmissionMissing => "CYNIC-BG-001",
            Self::BreakglassAdmissionInvalid => "CYNIC-BG-002",
            Self::BreakglassAdmissionExpired => "CYNIC-BG-003",
            Self::BreakglassActionForbidden => "CYNIC-BG-004",
            Self::BreakglassReceiptConsumed => "CYNIC-BG-005",
            Self::BreakglassAuthorityVerifierUnavailable => "CYNIC-BG-006",
            Self::BreakglassSignatureInvalid => "CYNIC-BG-007",
            Self::BreakglassConsumptionPersistFailed => "CYNIC-BG-008",
            Self::ActivationEpochUnobservable => "CYNIC-ACT-001",
            Self::ActivationBootstrapIncomplete => "CYNIC-ACT-002",
            Self::ActivationTakeoverProcessInactive => "CYNIC-ACT-003",
            Self::ActivationTakeoverIdentityUnverified => "CYNIC-ACT-004",
            Self::ActivationReadBackAbsent => "CYNIC-ACT-005",
            Self::ActivationConsentMissing => "CYNIC-ACT-006",
            Self::ActivationPhaseAuthorityInsufficient => "CYNIC-ACT-007",
            Self::AgentScopeConflict => "CYNIC-AGT-001",
            Self::AgentClaimExpired => "CYNIC-AGT-002",
            Self::AgentSharedTypeDrift => "CYNIC-AGT-003",
            Self::AgentMutationWithoutClaim => "CYNIC-AGT-004",
            Self::AgentCoordinationStoreUnavailable => "CYNIC-AGT-005",
            Self::AgentMergeQueueRace => "CYNIC-AGT-006",
            Self::AgentBudgetExceeded => "CYNIC-AGT-007",
            Self::FleetObservationBlackout => "CYNIC-FLT-001",
        }
    }

    pub const fn operator_action(self) -> &'static str {
        match self {
            Self::BuildAdmissionAuthorityMaterialMissing => {
                "restore the node-owned scheduler authority; do not run a local build fallback"
            }
            Self::BuildRunnerUnreachable | Self::NetworkBuildRunnerUnreachable => {
                "restore the declared runner transport, then read back its node observation"
            }
            Self::BuildSchedulerInactive => {
                "restore enrollment, scheduler capabilities, signing material, and durable state"
            }
            Self::BuildWorktreeDirty => {
                "submit an immutable source snapshot instead of admitting the working tree"
            }
            Self::BuildAdmissionVerifierUnavailable => {
                "install and observe the node-owned build grant verifier before scheduling work"
            }
            Self::BuildSourceCommitUnobservable => {
                "materialize an immutable checkout with an observable source commit"
            }
            Self::BuildNodeIdentityUnobservable => {
                "restore the declared node identity before requesting an admission"
            }
            Self::NetworkMeshTransportAbsent => {
                "restore the declared mesh or approved provider fallback before scheduling work"
            }
            Self::NetworkActuatorMissing => {
                "install the bounded network actuator through its privileged bootstrap plan"
            }
            Self::LxcMutationAuthorityUnverified => {
                "do not grant a mutation through the LXC bridge without a RecoveryBreakglass admission"
            }
            Self::BreakglassAdmissionMissing => {
                "obtain a time-bounded RecoveryBreakglass admission from the incident authority"
            }
            Self::BreakglassAdmissionInvalid => {
                "reject the request and issue a new admission bound to the exact target and action"
            }
            Self::BreakglassAdmissionExpired => {
                "issue a new short-lived admission after reconfirming incident scope"
            }
            Self::BreakglassActionForbidden => {
                "use only an action explicitly listed in the breakglass admission"
            }
            Self::BreakglassReceiptConsumed => {
                "do not reuse the receipt; inspect the recorded read-back or issue a new incident admission"
            }
            Self::BreakglassAuthorityVerifierUnavailable => {
                "do not execute; restore the incident authority verifier with atomic receipt consumption"
            }
            Self::BreakglassSignatureInvalid => {
                "reject the admission and obtain a newly signed incident authority receipt"
            }
            Self::BreakglassConsumptionPersistFailed => {
                "do not execute; restore the durable incident receipt store and read-back proof"
            }
            Self::ActivationEpochUnobservable => {
                "observe the active deployment epoch before attempting node activation"
            }
            Self::ActivationBootstrapIncomplete => {
                "complete federated bootstrap phases before attempting node activation"
            }
            Self::ActivationTakeoverProcessInactive => {
                "verify runtime takeover process readiness before promoting standby node"
            }
            Self::ActivationTakeoverIdentityUnverified => {
                "verify candidate node identity before granting runtime takeover promotion"
            }
            Self::ActivationReadBackAbsent => {
                "generate and record machine-readable read-back proof after node activation"
            }
            Self::ActivationConsentMissing => {
                "obtain operator or constitutional consent before applying node activation"
            }
            Self::ActivationPhaseAuthorityInsufficient => {
                "ensure runtime authority matches the required phase before activation transition"
            }
            Self::AgentScopeConflict => {
                "resolve overlapping agent scopes or select a non-conflicting hierarchy path"
            }
            Self::AgentClaimExpired => {
                "renew the agent coordination lease before proceeding with mutation"
            }
            Self::AgentSharedTypeDrift => {
                "reconcile shared schema and types with canonical contracts"
            }
            Self::AgentMutationWithoutClaim => {
                "acquire a valid coordination lease before mutating shared worktree resources"
            }
            Self::AgentCoordinationStoreUnavailable => {
                "restore access to the canonical coordination store (.agent-claims.json)"
            }
            Self::AgentBudgetExceeded => {
                "request a smaller budget or obtain a parent grant; budgets never self-amplify"
            }
            Self::AgentMergeQueueRace => {
                "rebase against current head and re-enqueue after resolving merge conflict"
            }
            Self::FleetObservationBlackout => {
                "restore fleet observation transport; no declared node is reporting, so health is unknown rather than healthy"
            }
        }
    }

    pub const fn retryable(self) -> bool {
        matches!(
            self,
            Self::BuildRunnerUnreachable
                | Self::BuildSchedulerInactive
                | Self::NetworkMeshTransportAbsent
                | Self::NetworkBuildRunnerUnreachable
                | Self::NetworkActuatorMissing
                | Self::BreakglassAdmissionExpired
                | Self::ActivationEpochUnobservable
                | Self::ActivationBootstrapIncomplete
                | Self::ActivationTakeoverProcessInactive
                | Self::ActivationReadBackAbsent
                | Self::AgentClaimExpired
                | Self::AgentCoordinationStoreUnavailable
                | Self::AgentMergeQueueRace
                | Self::FleetObservationBlackout
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CynicError {
    pub schema: String,
    /// Stable identifier intended for operators and automation.
    pub code: String,
    /// Typed classification retained for protocol consumers.
    pub class: CynicErrorCode,
    pub retryable: bool,
    pub operator_action: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

impl CynicError {
    pub fn new(code: CynicErrorCode) -> Self {
        Self {
            schema: ERROR_CATALOG_SCHEMA.to_string(),
            code: code.code().to_string(),
            class: code,
            retryable: code.retryable(),
            operator_action: code.operator_action().to_string(),
            evidence_refs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_remediation_are_stable() {
        let error = CynicError::new(CynicErrorCode::BreakglassAdmissionExpired);
        assert_eq!(error.schema, ERROR_CATALOG_SCHEMA);
        assert_eq!(error.code, "CYNIC-BG-003");
        assert!(error.retryable);
        assert!(!error.operator_action.is_empty());
    }

    /// Regression proof: no two variants may share a stable operator code.
    /// A duplicate would silently collapse two distinct violated contracts
    /// into one operator signal.
    #[test]
    fn every_code_is_unique() {
        let mut seen: Vec<&'static str> = Vec::new();
        for code in CYNIC_ERROR_CODES {
            let stable = code.code();
            assert!(
                !seen.contains(&stable),
                "duplicate error code {stable} for {code:?}"
            );
            seen.push(stable);
        }
        assert_eq!(seen.len(), CYNIC_ERROR_CODES.len());
    }

    /// Regression proof: every variant carries a non-empty operator action.
    /// An empty remediation turns a typed code back into an opaque string.
    #[test]
    fn every_code_has_operator_action() {
        for code in CYNIC_ERROR_CODES {
            assert!(
                !code.operator_action().is_empty(),
                "missing operator action for {code:?}"
            );
        }
    }

    /// Activation codes are the typed replacement for ad-hoc blocker strings
    /// such as "takeover_activation_process_inactive".
    #[test]
    fn activation_codes_are_catalogued() {
        let activation = [
            (CynicErrorCode::ActivationEpochUnobservable, "CYNIC-ACT-001"),
            (
                CynicErrorCode::ActivationBootstrapIncomplete,
                "CYNIC-ACT-002",
            ),
            (
                CynicErrorCode::ActivationTakeoverProcessInactive,
                "CYNIC-ACT-003",
            ),
            (
                CynicErrorCode::ActivationTakeoverIdentityUnverified,
                "CYNIC-ACT-004",
            ),
            (CynicErrorCode::ActivationReadBackAbsent, "CYNIC-ACT-005"),
            (CynicErrorCode::ActivationConsentMissing, "CYNIC-ACT-006"),
            (
                CynicErrorCode::ActivationPhaseAuthorityInsufficient,
                "CYNIC-ACT-007",
            ),
        ];
        for (variant, expected) in activation {
            assert_eq!(variant.code(), expected);
            assert!(CYNIC_ERROR_CODES.contains(&variant));
        }
    }

    /// Agent emergence codes make multi-agent worktree collisions typed data
    /// instead of repeated compiler symptoms.
    #[test]
    fn agent_emergence_codes_are_catalogued() {
        let emergence = [
            (CynicErrorCode::AgentScopeConflict, "CYNIC-AGT-001"),
            (CynicErrorCode::AgentClaimExpired, "CYNIC-AGT-002"),
            (CynicErrorCode::AgentSharedTypeDrift, "CYNIC-AGT-003"),
            (CynicErrorCode::AgentMutationWithoutClaim, "CYNIC-AGT-004"),
            (
                CynicErrorCode::AgentCoordinationStoreUnavailable,
                "CYNIC-AGT-005",
            ),
            (CynicErrorCode::AgentMergeQueueRace, "CYNIC-AGT-006"),
            (CynicErrorCode::AgentBudgetExceeded, "CYNIC-AGT-007"),
        ];
        for (variant, expected) in emergence {
            assert_eq!(variant.code(), expected);
            assert!(CYNIC_ERROR_CODES.contains(&variant));
        }
    }

    /// A scope conflict is not retryable by waiting: the agent must pick a
    /// different scope. A claim expiry is retryable after renewal.
    #[test]
    fn agent_retryability_distinguishes_conflict_from_expiry() {
        assert!(!CynicError::new(CynicErrorCode::AgentScopeConflict).retryable);
        assert!(!CynicError::new(CynicErrorCode::AgentMutationWithoutClaim).retryable);
        assert!(CynicError::new(CynicErrorCode::AgentClaimExpired).retryable);
    }
}
