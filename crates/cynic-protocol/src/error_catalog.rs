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
        }
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
            retryable: matches!(
                code,
                CynicErrorCode::BuildRunnerUnreachable
                    | CynicErrorCode::BuildSchedulerInactive
                    | CynicErrorCode::NetworkMeshTransportAbsent
                    | CynicErrorCode::NetworkBuildRunnerUnreachable
                    | CynicErrorCode::NetworkActuatorMissing
                    | CynicErrorCode::BreakglassAdmissionExpired
            ),
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
}
