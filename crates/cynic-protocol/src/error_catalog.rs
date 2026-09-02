use serde::{Deserialize, Serialize};

/// Stable, machine-readable operator error catalogue.
///
/// A code identifies the failed contract; reports still carry contextual
/// evidence and lower-level blockers separately. It deliberately contains no
/// secrets, paths, or provider-specific command output.
pub const ERROR_CATALOG_SCHEMA: &str = "cynic.error_catalog.v1";

/// Sovereign error domain — the first-class classification axis.
///
/// Each domain owns a stable short prefix (`BLD`, `NET`, ...) that is derived
/// from the enum, never hardcoded per-code. Domains are the canonical
/// grouping used by operator projections, dashboards, and alert routing; a
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CynicErrorDomain {
    Build,
    Network,
    Lxc,
    Breakglass,
    Activation,
    Agent,
    Fleet,
    OsHealth,
    Storage,
    /// Toolchain install provenance, binary resolution, mutator authority.
    Toolchain,
    /// Publication consent, scope boundaries, unwhitelisted field disclosure.
    Publication,
    /// Organic opportunities observation, value frames, decision lineage.
    Opportunity,
    /// Runtime storage: disk full, capacity pressure, extension required.
    Runtime,
}

impl CynicErrorDomain {
    /// Stable short prefix carried in the human/operator code (`CYNIC-BLD-001`).
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Build => "BLD",
            Self::Network => "NET",
            Self::Lxc => "LXC",
            Self::Breakglass => "BG",
            Self::Activation => "ACT",
            Self::Agent => "AGT",
            Self::Fleet => "FLT",
            Self::OsHealth => "OSH",
            Self::Storage => "STO",
            Self::Toolchain => "TL",
            Self::Publication => "PUB",
            Self::Opportunity => "OPP",
            Self::Runtime => "RT",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Network => "network",
            Self::Lxc => "lxc",
            Self::Breakglass => "breakglass",
            Self::Activation => "activation",
            Self::Agent => "agent",
            Self::Fleet => "fleet",
            Self::OsHealth => "os_health",
            Self::Storage => "storage",
            Self::Toolchain => "toolchain",
            Self::Publication => "publication",
            Self::Opportunity => "opportunity",
            Self::Runtime => "runtime",
        }
    }
}

/// Sovereign error nature — the second classification axis.
///
/// Distinct from severity: two codes may both be `critical` but one is a hard
/// `Blocked` (no path forward without an authority escalation) while the other
/// is `Recoverable` (retry or bounded remediation). This axis is what makes
/// "treat blockers as typed reconciliation or authority escalation, never
/// manual bypass" operational.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CynicErrorClass {
    /// No safe forward path without an authority escalation (admission grant,
    /// breakglass, constitutional consent). Waiting does not help.
    Blocked,
    /// Resolvable by a bounded, reviewable remediation or by retry after a
    /// dependency is restored. Never a silent bypass.
    Recoverable,
    /// Requires operator or constitutional consent before any action.
    Constitutional,
    /// Multi-agent collision; resolved by scope selection or a parent grant,
    /// not by retrying the same claim.
    AgentResolution,
}

impl CynicErrorClass {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Recoverable => "recoverable",
            Self::Constitutional => "constitutional",
            Self::AgentResolution => "agent_resolution",
        }
    }
}

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
    /// A registry claim about a node contradicts what the substrate reports.
    ///
    /// The registry is desired state, never proof. When it names a role, an
    /// address, or a service that observation refutes, the divergence itself is
    /// the finding — silently trusting either side fabricates convergence.
    FleetRegistrySubstrateDivergence,
    /// One name resolves to different addresses depending on who asks.
    ///
    /// Per-node `/etc/hosts` overrides make a name locally correct and globally
    /// inconsistent, so peers disagree about where a role lives without any
    /// single node observing a fault.
    FleetNameResolutionInconsistent,
    /// A node runs a fleet-facing role while being unreachable by name.
    ///
    /// Reachable-by-IP-only means every peer needs a hardcoded address, which
    /// is how per-node overrides start.
    FleetNodeUndiscoverable,
    /// OS-level anomaly codes (CYNIC-OSH-*).
    ///
    /// These make the 2026-08-18 crash class typed data instead of opaque
    /// strings: soft lockup without panic, THP defrag stalls, memory reclaim
    /// pressure, kernel-log evidence of freeze events, and unreadable signals.
    /// The protocol crate carries only the codes; the host adapter observes the
    /// substrate behind /proc and /sys.
    OsHealthSoftlockupPanicDisabled,
    OsHealthThpEnabledRisky,
    OsHealthThpDefragRisky,
    OsHealthMemAvailBelowFloor,
    OsHealthReclaimPressureAboveCeiling,
    OsHealthKernelSoftlockupDetected,
    OsHealthKernelHungTaskDetected,
    OsHealthOomDetected,
    OsHealthUnknown,
    /// The kernel anomaly is real but no lower-layer code was consumed to
    /// confirm a specific root cause — observation is partial.
    OsHealthObservationUnknown,
    ServiceCrashLoop,
    StorageEndpointUnreachable,
    StorageTopologyReplicaMismatch,
    StorageNullDegradedWriteRefused,
    StoragePoolExhausted,
    // ── Toolchain domain (CYNIC-TL-*) ──
    ToolchainProvenanceUnverified,
    ToolchainBinaryMissing,
    ToolchainMutatorAuthorityMismatch,
    // ── Publication domain (CYNIC-PUB-*) ──
    PublicationConsentMissing,
    PublicationConsentExpired,
    PublicationFieldUnwhitelisted,
    // ── Opportunity domain (CYNIC-OPP-*) ──
    OpportunitySourceUnreadable,
    OpportunityValueFrameInvalid,
    OpportunityDecisionDigestMismatch,
    RuntimeDiskFull,
    RuntimeCapacityPressure,
    RuntimeOrphanedCheckout,
    RuntimeStorageExtensionRequired,
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
    CynicErrorCode::FleetRegistrySubstrateDivergence,
    CynicErrorCode::FleetNameResolutionInconsistent,
    CynicErrorCode::FleetNodeUndiscoverable,
    CynicErrorCode::OsHealthSoftlockupPanicDisabled,
    CynicErrorCode::OsHealthThpEnabledRisky,
    CynicErrorCode::OsHealthThpDefragRisky,
    CynicErrorCode::OsHealthMemAvailBelowFloor,
    CynicErrorCode::OsHealthReclaimPressureAboveCeiling,
    CynicErrorCode::OsHealthKernelSoftlockupDetected,
    CynicErrorCode::OsHealthKernelHungTaskDetected,
    CynicErrorCode::OsHealthOomDetected,
    CynicErrorCode::OsHealthUnknown,
    CynicErrorCode::OsHealthObservationUnknown,
    CynicErrorCode::ServiceCrashLoop,
    CynicErrorCode::StorageEndpointUnreachable,
    CynicErrorCode::StorageTopologyReplicaMismatch,
    CynicErrorCode::StorageNullDegradedWriteRefused,
    CynicErrorCode::StoragePoolExhausted,
    CynicErrorCode::ToolchainProvenanceUnverified,
    CynicErrorCode::ToolchainBinaryMissing,
    CynicErrorCode::ToolchainMutatorAuthorityMismatch,
    CynicErrorCode::PublicationConsentMissing,
    CynicErrorCode::PublicationConsentExpired,
    CynicErrorCode::PublicationFieldUnwhitelisted,
    CynicErrorCode::OpportunitySourceUnreadable,
    CynicErrorCode::OpportunityValueFrameInvalid,
    CynicErrorCode::OpportunityDecisionDigestMismatch,
    CynicErrorCode::RuntimeDiskFull,
    CynicErrorCode::RuntimeCapacityPressure,
    CynicErrorCode::RuntimeOrphanedCheckout,
    CynicErrorCode::RuntimeStorageExtensionRequired,
];

impl CynicErrorCode {
    /// Stable two-axis classification: domain + sequential index within the
    /// domain. The operator string `CYNIC-{PREFIX}-{NNN}` is derived from this
    /// (the prefix is never hardcoded per-code).
    pub const fn domain(self) -> CynicErrorDomain {
        match self {
            Self::BuildAdmissionAuthorityMaterialMissing
            | Self::BuildRunnerUnreachable
            | Self::BuildSchedulerInactive
            | Self::BuildWorktreeDirty
            | Self::BuildAdmissionVerifierUnavailable
            | Self::BuildSourceCommitUnobservable
            | Self::BuildNodeIdentityUnobservable => CynicErrorDomain::Build,
            Self::NetworkMeshTransportAbsent
            | Self::NetworkBuildRunnerUnreachable
            | Self::NetworkActuatorMissing => CynicErrorDomain::Network,
            Self::LxcMutationAuthorityUnverified => CynicErrorDomain::Lxc,
            Self::BreakglassAdmissionMissing
            | Self::BreakglassAdmissionInvalid
            | Self::BreakglassAdmissionExpired
            | Self::BreakglassActionForbidden
            | Self::BreakglassReceiptConsumed
            | Self::BreakglassAuthorityVerifierUnavailable
            | Self::BreakglassSignatureInvalid
            | Self::BreakglassConsumptionPersistFailed => CynicErrorDomain::Breakglass,
            Self::ActivationEpochUnobservable
            | Self::ActivationBootstrapIncomplete
            | Self::ActivationTakeoverProcessInactive
            | Self::ActivationTakeoverIdentityUnverified
            | Self::ActivationReadBackAbsent
            | Self::ActivationConsentMissing
            | Self::ActivationPhaseAuthorityInsufficient => CynicErrorDomain::Activation,
            Self::AgentScopeConflict
            | Self::AgentClaimExpired
            | Self::AgentSharedTypeDrift
            | Self::AgentMutationWithoutClaim
            | Self::AgentCoordinationStoreUnavailable
            | Self::AgentMergeQueueRace
            | Self::AgentBudgetExceeded => CynicErrorDomain::Agent,
            Self::FleetObservationBlackout
            | Self::FleetRegistrySubstrateDivergence
            | Self::FleetNameResolutionInconsistent
            | Self::FleetNodeUndiscoverable => CynicErrorDomain::Fleet,
            Self::OsHealthSoftlockupPanicDisabled
            | Self::OsHealthThpEnabledRisky
            | Self::OsHealthThpDefragRisky
            | Self::OsHealthMemAvailBelowFloor
            | Self::OsHealthReclaimPressureAboveCeiling
            | Self::OsHealthKernelSoftlockupDetected
            | Self::OsHealthKernelHungTaskDetected
            | Self::OsHealthOomDetected
            | Self::OsHealthUnknown
            | Self::OsHealthObservationUnknown
            | Self::ServiceCrashLoop => CynicErrorDomain::OsHealth,
            Self::StorageEndpointUnreachable
            | Self::StorageTopologyReplicaMismatch
            | Self::StorageNullDegradedWriteRefused
            | Self::StoragePoolExhausted => CynicErrorDomain::Storage,
            Self::ToolchainProvenanceUnverified
            | Self::ToolchainBinaryMissing
            | Self::ToolchainMutatorAuthorityMismatch => CynicErrorDomain::Toolchain,
            Self::PublicationConsentMissing
            | Self::PublicationConsentExpired
            | Self::PublicationFieldUnwhitelisted => CynicErrorDomain::Publication,
            Self::OpportunitySourceUnreadable
            | Self::OpportunityValueFrameInvalid
            | Self::OpportunityDecisionDigestMismatch => CynicErrorDomain::Opportunity,
            Self::RuntimeDiskFull
            | Self::RuntimeCapacityPressure
            | Self::RuntimeOrphanedCheckout
            | Self::RuntimeStorageExtensionRequired => CynicErrorDomain::Runtime,
        }
    }

    /// Nature axis: blocked / recoverable / constitutional / agent_resolution.
    pub const fn class(self) -> CynicErrorClass {
        match self {
            // Authority material, verifier, identity: no safe path without escalation.
            Self::BuildAdmissionAuthorityMaterialMissing
            | Self::BuildAdmissionVerifierUnavailable
            | Self::BuildSourceCommitUnobservable
            | Self::BuildNodeIdentityUnobservable
            | Self::LxcMutationAuthorityUnverified
            | Self::BreakglassAdmissionMissing
            | Self::BreakglassAuthorityVerifierUnavailable
            | Self::BreakglassSignatureInvalid
            | Self::BreakglassConsumptionPersistFailed
            | Self::BreakglassAdmissionInvalid
            | Self::BreakglassActionForbidden
            | Self::BreakglassReceiptConsumed
            | Self::ActivationTakeoverIdentityUnverified
            | Self::ActivationPhaseAuthorityInsufficient
            | Self::AgentMutationWithoutClaim
            | Self::AgentCoordinationStoreUnavailable
            | Self::AgentBudgetExceeded
            | Self::FleetRegistrySubstrateDivergence
            | Self::FleetNameResolutionInconsistent
            | Self::FleetNodeUndiscoverable
            | Self::StorageNullDegradedWriteRefused
            | Self::ToolchainProvenanceUnverified
            | Self::ToolchainMutatorAuthorityMismatch
            | Self::PublicationFieldUnwhitelisted
            | Self::OpportunityValueFrameInvalid
            | Self::OpportunityDecisionDigestMismatch => CynicErrorClass::Blocked,

            Self::OsHealthOomDetected
            | Self::ServiceCrashLoop
            | Self::ActivationConsentMissing
            | Self::PublicationConsentMissing => CynicErrorClass::Constitutional,

            // Multi-agent collision, not a retry.
            Self::AgentScopeConflict | Self::AgentSharedTypeDrift | Self::AgentMergeQueueRace => {
                CynicErrorClass::AgentResolution
            }

            // Everything else: restore dependency or apply bounded remediation.
            Self::BuildRunnerUnreachable
            | Self::BuildSchedulerInactive
            | Self::BuildWorktreeDirty
            | Self::NetworkMeshTransportAbsent
            | Self::NetworkBuildRunnerUnreachable
            | Self::NetworkActuatorMissing
            | Self::BreakglassAdmissionExpired
            | Self::ActivationEpochUnobservable
            | Self::ActivationBootstrapIncomplete
            | Self::ActivationTakeoverProcessInactive
            | Self::ActivationReadBackAbsent
            | Self::AgentClaimExpired
            | Self::FleetObservationBlackout
            | Self::OsHealthSoftlockupPanicDisabled
            | Self::OsHealthThpEnabledRisky
            | Self::OsHealthThpDefragRisky
            | Self::OsHealthMemAvailBelowFloor
            | Self::OsHealthReclaimPressureAboveCeiling
            | Self::OsHealthKernelSoftlockupDetected
            | Self::OsHealthKernelHungTaskDetected
            | Self::OsHealthUnknown
            | Self::OsHealthObservationUnknown
            | Self::StorageEndpointUnreachable
            | Self::StorageTopologyReplicaMismatch
            | Self::StoragePoolExhausted
            | Self::ToolchainBinaryMissing
            | Self::PublicationConsentExpired
            | Self::OpportunitySourceUnreadable => CynicErrorClass::Recoverable,
            Self::RuntimeDiskFull
            | Self::RuntimeCapacityPressure
            | Self::RuntimeOrphanedCheckout
            | Self::RuntimeStorageExtensionRequired => CynicErrorClass::Recoverable,
        }
    }

    /// Stable operator code `CYNIC-{PREFIX}-{NNN}` derived from the domain
    /// prefix and the intra-domain sequence. The sequence is assigned by
    /// `domain_index` so the two never drift.
    pub const fn code(self) -> &'static str {
        match (self.domain(), self.domain_index()) {
            (CynicErrorDomain::Build, 1) => "CYNIC-BLD-001",
            (CynicErrorDomain::Build, 2) => "CYNIC-BLD-002",
            (CynicErrorDomain::Build, 3) => "CYNIC-BLD-003",
            (CynicErrorDomain::Build, 4) => "CYNIC-BLD-004",
            (CynicErrorDomain::Build, 5) => "CYNIC-BLD-005",
            (CynicErrorDomain::Build, 6) => "CYNIC-BLD-006",
            (CynicErrorDomain::Build, 7) => "CYNIC-BLD-007",
            (CynicErrorDomain::Network, 1) => "CYNIC-NET-001",
            (CynicErrorDomain::Network, 2) => "CYNIC-NET-002",
            (CynicErrorDomain::Network, 3) => "CYNIC-NET-003",
            (CynicErrorDomain::Lxc, 1) => "CYNIC-LXC-001",
            (CynicErrorDomain::Breakglass, 1) => "CYNIC-BG-001",
            (CynicErrorDomain::Breakglass, 2) => "CYNIC-BG-002",
            (CynicErrorDomain::Breakglass, 3) => "CYNIC-BG-003",
            (CynicErrorDomain::Breakglass, 4) => "CYNIC-BG-004",
            (CynicErrorDomain::Breakglass, 5) => "CYNIC-BG-005",
            (CynicErrorDomain::Breakglass, 6) => "CYNIC-BG-006",
            (CynicErrorDomain::Breakglass, 7) => "CYNIC-BG-007",
            (CynicErrorDomain::Breakglass, 8) => "CYNIC-BG-008",
            (CynicErrorDomain::Activation, 1) => "CYNIC-ACT-001",
            (CynicErrorDomain::Activation, 2) => "CYNIC-ACT-002",
            (CynicErrorDomain::Activation, 3) => "CYNIC-ACT-003",
            (CynicErrorDomain::Activation, 4) => "CYNIC-ACT-004",
            (CynicErrorDomain::Activation, 5) => "CYNIC-ACT-005",
            (CynicErrorDomain::Activation, 6) => "CYNIC-ACT-006",
            (CynicErrorDomain::Activation, 7) => "CYNIC-ACT-007",
            (CynicErrorDomain::Agent, 1) => "CYNIC-AGT-001",
            (CynicErrorDomain::Agent, 2) => "CYNIC-AGT-002",
            (CynicErrorDomain::Agent, 3) => "CYNIC-AGT-003",
            (CynicErrorDomain::Agent, 4) => "CYNIC-AGT-004",
            (CynicErrorDomain::Agent, 5) => "CYNIC-AGT-005",
            (CynicErrorDomain::Agent, 6) => "CYNIC-AGT-006",
            (CynicErrorDomain::Agent, 7) => "CYNIC-AGT-007",
            (CynicErrorDomain::Fleet, 1) => "CYNIC-FLT-001",
            (CynicErrorDomain::Fleet, 2) => "CYNIC-FLT-002",
            (CynicErrorDomain::Fleet, 3) => "CYNIC-FLT-003",
            (CynicErrorDomain::Fleet, 4) => "CYNIC-FLT-004",
            (CynicErrorDomain::OsHealth, 1) => "CYNIC-OSH-001",
            (CynicErrorDomain::OsHealth, 2) => "CYNIC-OSH-002",
            (CynicErrorDomain::OsHealth, 3) => "CYNIC-OSH-003",
            (CynicErrorDomain::OsHealth, 4) => "CYNIC-OSH-004",
            (CynicErrorDomain::OsHealth, 5) => "CYNIC-OSH-005",
            (CynicErrorDomain::OsHealth, 6) => "CYNIC-OSH-006",
            (CynicErrorDomain::OsHealth, 7) => "CYNIC-OSH-007",
            (CynicErrorDomain::OsHealth, 8) => "CYNIC-OSH-008",
            (CynicErrorDomain::OsHealth, 9) => "CYNIC-OSH-009",
            (CynicErrorDomain::OsHealth, 10) => "CYNIC-OSH-010",
            (CynicErrorDomain::OsHealth, 11) => "CYNIC-OSH-011",
            (CynicErrorDomain::Storage, 1) => "CYNIC-STO-001",
            (CynicErrorDomain::Storage, 2) => "CYNIC-STO-002",
            (CynicErrorDomain::Storage, 3) => "CYNIC-STO-003",
            (CynicErrorDomain::Storage, 4) => "CYNIC-STO-004",
            (CynicErrorDomain::Toolchain, 1) => "CYNIC-TL-001",
            (CynicErrorDomain::Toolchain, 2) => "CYNIC-TL-002",
            (CynicErrorDomain::Toolchain, 3) => "CYNIC-TL-003",
            (CynicErrorDomain::Publication, 1) => "CYNIC-PUB-001",
            (CynicErrorDomain::Publication, 2) => "CYNIC-PUB-002",
            (CynicErrorDomain::Publication, 3) => "CYNIC-PUB-003",
            (CynicErrorDomain::Opportunity, 1) => "CYNIC-OPP-001",
            (CynicErrorDomain::Opportunity, 2) => "CYNIC-OPP-002",
            (CynicErrorDomain::Opportunity, 3) => "CYNIC-OPP-003",
            (CynicErrorDomain::Runtime, 1) => "CYNIC-RT-001",
            (CynicErrorDomain::Runtime, 2) => "CYNIC-RT-002",
            (CynicErrorDomain::Runtime, 3) => "CYNIC-RT-003",
            (CynicErrorDomain::Runtime, 4) => "CYNIC-RT-004",
            _ => "CYNIC-ERR-000",
        }
    }

    /// 1-based index of this code within its domain. Used to derive the stable
    /// operator code so addition order cannot silently drift the number.
    pub const fn domain_index(self) -> u8 {
        let mut idx = 0u8;
        let mut i = 0;
        while i < CYNIC_ERROR_CODES.len() {
            if CYNIC_ERROR_CODES[i].domain() as u8 == self.domain() as u8 {
                idx += 1;
                if CYNIC_ERROR_CODES[i] as u8 == self as u8 {
                    return idx;
                }
            }
            i += 1;
        }
        0
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
            Self::FleetRegistrySubstrateDivergence => {
                "reconcile the registry entry with the observed substrate, or materialize the declared state; do not trust either side silently"
            }
            Self::FleetNameResolutionInconsistent => {
                "publish the name from one authoritative zone and remove per-node /etc/hosts overrides that disagree"
            }
            Self::FleetNodeUndiscoverable => {
                "give the node an address and DNS record on the plane its role serves, so peers stop hardcoding its IP"
            }
            Self::OsHealthSoftlockupPanicDisabled => {
                "sysctl -w kernel.softlockup_panic=1 and persist in /etc/sysctl.d so a stuck CPU reboots instead of freezing the host"
            }
            Self::OsHealthThpEnabledRisky => {
                "set /sys/kernel/mm/transparent_hugepage/enabled to madvise or never"
            }
            Self::OsHealthThpDefragRisky => {
                "set /sys/kernel/mm/transparent_hugepage/defrag to madvise, never, or defer-only"
            }
            Self::OsHealthMemAvailBelowFloor => {
                "reduce concurrent memory-heavy workloads or raise available memory headroom"
            }
            Self::OsHealthReclaimPressureAboveCeiling => {
                "inspect memory consumers via /proc/meminfo and cgroup pressure; coordinate workload concurrency via the runtime substrate, not ad-hoc kills"
            }
            Self::OsHealthKernelSoftlockupDetected => {
                "correlate with THP and memory findings; treat kernel log evidence as substrate fault read-back, not a CYNIC bug"
            }
            Self::OsHealthKernelHungTaskDetected => {
                "investigate the blocking subsystem in the kernel log; likely memory reclaim or I/O stall"
            }
            Self::OsHealthOomDetected => {
                "raise headroom or constrain workloads; OOM indicates the memory pressure ceiling was crossed"
            }
            Self::OsHealthUnknown => {
                "the anomaly is recognised but unsourced — emit the raw observation and classify the substrate contract violation"
            }
            Self::OsHealthObservationUnknown => {
                "observation is partial: confirm which /proc or /sys probe is unreadable and report the residual unknown"
            }
            Self::ServiceCrashLoop => {
                "inspect service logs and backpressure before restarting unit"
            }
            Self::StorageEndpointUnreachable => {
                "restore network reachability to declared storage endpoint or promote a replica"
            }
            Self::StorageTopologyReplicaMismatch => {
                "reconcile live storage endpoints with declared replica target; do not assume single node"
            }
            Self::StorageNullDegradedWriteRefused => {
                "storage is in degraded null mode; mutations are rejected to prevent silent data loss"
            }
            Self::StoragePoolExhausted => {
                "expand storage pool capacity or prune expired ephemeral artifacts"
            }
            Self::ToolchainProvenanceUnverified => {
                "verify install provenance channel; do not assume universal self-update commands"
            }
            Self::ToolchainBinaryMissing => {
                "install required toolchain binary via declared provenance channel"
            }
            Self::ToolchainMutatorAuthorityMismatch => {
                "execute repair with the declared mutator authority (e.g. uv tool vs uv pip --system)"
            }
            Self::PublicationConsentMissing => {
                "obtain explicit opt-in publication consent before projecting data to public audience"
            }
            Self::PublicationConsentExpired => {
                "renew publication consent; policy version digest drift invalidates prior consent"
            }
            Self::PublicationFieldUnwhitelisted => {
                "field is not in disclosure whitelist; private data cannot be projected"
            }
            Self::OpportunitySourceUnreadable => {
                "source trace or worktree is unreadable; report as unavailable rather than empty"
            }
            Self::OpportunityValueFrameInvalid => {
                "value frame tie-break dimensions are invalid; provide a valid ranking frame"
            }
            Self::OpportunityDecisionDigestMismatch => {
                "decision digest does not match target opportunity; verify opportunity identity"
            }
            Self::RuntimeDiskFull => {
                "extend the VM disk via hypervisor API or free orphaned resources"
            }
            Self::RuntimeCapacityPressure => {
                "review resource usage and extend capacity or migrate workloads"
            }
            Self::RuntimeOrphanedCheckout => {
                "remove orphaned builder checkouts via OneShotCapability"
            }
            Self::RuntimeStorageExtensionRequired => {
                "request a disk extension via the hypervisor supervision port"
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
                | Self::ServiceCrashLoop
                | Self::StorageEndpointUnreachable
                | Self::StorageTopologyReplicaMismatch
                | Self::StoragePoolExhausted
                | Self::ToolchainBinaryMissing
                | Self::PublicationConsentExpired
                | Self::OpportunitySourceUnreadable
        )
    }
}

/// All codes belonging to a domain — used by operator projections and alert
/// routing. Returns a fresh slice view; the caller iterates without cloning.
impl CynicErrorDomain {
    pub fn codes(self) -> Vec<CynicErrorCode> {
        CYNIC_ERROR_CODES
            .iter()
            .copied()
            .filter(|c| c.domain() == self)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CynicError {
    pub schema: String,
    /// Stable identifier intended for operators and automation.
    pub code: String,
    /// Typed classification retained for protocol consumers.
    pub class: CynicErrorCode,
    /// Domain axis (first-class grouping).
    pub domain: CynicErrorDomain,
    /// Nature axis (blocked / recoverable / constitutional / agent_resolution).
    pub nature: CynicErrorClass,
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
            domain: code.domain(),
            nature: code.class(),
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
        assert_eq!(error.domain, CynicErrorDomain::Breakglass);
        assert_eq!(error.nature, CynicErrorClass::Recoverable);
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

    /// The code string is fully derived from (domain, domain_index): the
    /// prefix must match the domain and the number must match the position.
    #[test]
    fn code_derives_from_domain_and_index() {
        for code in CYNIC_ERROR_CODES {
            let expected = format!(
                "CYNIC-{}-{:03}",
                code.domain().prefix(),
                code.domain_index()
            );
            assert_eq!(
                code.code(),
                expected,
                "code string drift for {code:?}: got {}, expected {expected}",
                code.code()
            );
        }
    }

    /// Domain axis is exhaustive and never the fallback `CYNIC-ERR-000`.
    #[test]
    fn every_code_resolves_to_a_domain() {
        for code in CYNIC_ERROR_CODES {
            assert_ne!(code.code(), "CYNIC-ERR-000", "unclassified code {code:?}");
        }
    }

    /// Nature axis is exhaustive.
    #[test]
    fn every_code_resolves_to_a_class() {
        for code in CYNIC_ERROR_CODES {
            let _ = code.class();
        }
    }

    /// `by_domain` returns exactly the codes of that domain.
    #[test]
    fn domain_codes_filter_is_complete() {
        for domain in [
            CynicErrorDomain::Build,
            CynicErrorDomain::Network,
            CynicErrorDomain::Lxc,
            CynicErrorDomain::Breakglass,
            CynicErrorDomain::Activation,
            CynicErrorDomain::Agent,
            CynicErrorDomain::Fleet,
            CynicErrorDomain::OsHealth,
        ] {
            let subset = domain.codes();
            for code in &subset {
                assert_eq!(code.domain(), domain);
            }
            assert_eq!(subset.len() as u8, domain_index_count(domain));
        }
    }

    fn domain_index_count(domain: CynicErrorDomain) -> u8 {
        CYNIC_ERROR_CODES
            .iter()
            .filter(|c| c.domain() == domain)
            .count() as u8
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
            assert_eq!(variant.domain(), CynicErrorDomain::Activation);
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

    /// OS health codes are the typed replacement for the strings
    /// previously scattered as "CYNIC-OSH-*" literals in host and protocol.
    /// They make the 2026-08-18 crash class typed data.
    #[test]
    fn os_health_codes_are_catalogued() {
        let osh = [
            (
                CynicErrorCode::OsHealthSoftlockupPanicDisabled,
                "CYNIC-OSH-001",
            ),
            (CynicErrorCode::OsHealthThpEnabledRisky, "CYNIC-OSH-002"),
            (CynicErrorCode::OsHealthThpDefragRisky, "CYNIC-OSH-003"),
            (CynicErrorCode::OsHealthMemAvailBelowFloor, "CYNIC-OSH-004"),
            (
                CynicErrorCode::OsHealthReclaimPressureAboveCeiling,
                "CYNIC-OSH-005",
            ),
            (
                CynicErrorCode::OsHealthKernelSoftlockupDetected,
                "CYNIC-OSH-006",
            ),
            (
                CynicErrorCode::OsHealthKernelHungTaskDetected,
                "CYNIC-OSH-007",
            ),
            (CynicErrorCode::OsHealthOomDetected, "CYNIC-OSH-008"),
            (CynicErrorCode::OsHealthUnknown, "CYNIC-OSH-009"),
            (CynicErrorCode::OsHealthObservationUnknown, "CYNIC-OSH-010"),
        ];
        for (variant, expected) in osh {
            assert_eq!(variant.code(), expected);
            assert!(CYNIC_ERROR_CODES.contains(&variant));
            assert_eq!(variant.domain(), CynicErrorDomain::OsHealth);
        }
    }

    /// OS health anomalies are not retryable by waiting — they require a
    /// corrective sysctl or workload-constraint action.
    #[test]
    fn os_health_codes_are_not_retryable() {
        assert!(!CynicErrorCode::OsHealthSoftlockupPanicDisabled.retryable());
        assert!(!CynicErrorCode::OsHealthThpEnabledRisky.retryable());
        assert!(!CynicErrorCode::OsHealthMemAvailBelowFloor.retryable());
        assert!(!CynicErrorCode::OsHealthKernelSoftlockupDetected.retryable());
    }
}
