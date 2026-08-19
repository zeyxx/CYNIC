//! Durable record of observed runtime emergences.
//!
//! "Every bug is evidence of an unmodelled contract." This module is where
//! that principle becomes machine-readable: each recorded emergence names the
//! canonical error code it violated, the contract layer that owns the repair,
//! and whether a regression proof exists.
//!
//! It deliberately does *not* classify prose. Classification happens once, at
//! the layer that owns the observation (`node_activation::classify_activation_blocker`,
//! `health_signal::HealthSignalPayload::error_codes`), and this gallery records
//! the already-typed result. A gallery that re-derived severity from a symptom
//! string would be a second decision source for the same concern.
//!
//! Persistence is content-addressed through `evidence::EvidenceEnvelope`, so a
//! gallery entry is an evidence payload like any other observation — not a new
//! parallel storage model.

use serde::{Deserialize, Serialize};

use crate::error_catalog::CynicErrorCode;
use crate::evidence::{EvidenceEnvelope, build_evidence_envelope};

pub const EMERGENCE_GALLERY_ENTRY_SCHEMA: &str = "cynic.emergence_gallery_entry.v1";

/// The contract layer that owns the repair for an emergence.
///
/// From the Coherence Pattern: "Repair flows downward to the violated
/// contract." Naming the layer prevents the common failure of patching a
/// symptom in a projection (CLI, report) when the defect lives in a port,
/// adapter, or policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractLayer {
    /// The intent could not be represented or compared.
    DesiredState,
    /// The substrate could not be read without mutation.
    Observation,
    /// The pure decision from desired and observed state was wrong or absent.
    Reconciliation,
    /// The mutation boundary (argv, privilege, consent, rollback) was violated.
    ExecutionBoundary,
    /// Read-back evidence was absent, ambiguous, or misinterpreted.
    Proof,
}

impl ContractLayer {
    /// Whether a defect at this layer can present a failure as success.
    ///
    /// Proof and observation defects are the dangerous class: they corrupt the
    /// evidence the operator relies on, so a false green is possible. A
    /// reconciliation or execution defect fails visibly instead.
    pub fn can_fabricate_convergence(self) -> bool {
        matches!(self, Self::Proof | Self::Observation)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DesiredState => "desired_state",
            Self::Observation => "observation",
            Self::Reconciliation => "reconciliation",
            Self::ExecutionBoundary => "execution_boundary",
            Self::Proof => "proof",
        }
    }
}

impl std::fmt::Display for ContractLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Repair status of a recorded emergence.
///
/// `Repaired` requires a regression proof reference: a claim of repair without
/// a test is not a repair, it is a memory. This is why the variant carries the
/// proof rather than a boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RepairStatus {
    /// Observed, root cause not yet identified.
    Observed,
    /// Root cause known, repair not applied.
    Diagnosed { root_cause: String },
    /// Repaired at the owning layer, with a regression proof.
    Repaired {
        root_cause: String,
        /// Test or verification command that fails if the defect returns.
        regression_proof: String,
        /// The layer where the repair was actually applied.
        repaired_at: ContractLayer,
    },
    /// Contained at a higher layer as an explicit temporary measure.
    ///
    /// AGENTS.md permits this only with residual risk and a removal condition
    /// recorded in machine-readable evidence — both are required fields here.
    Contained {
        residual_risk: String,
        removal_condition: String,
    },
}

impl RepairStatus {
    /// Whether this emergence still carries unproven risk.
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Repaired { .. })
    }
}

/// One recorded runtime emergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergenceGalleryEntry {
    pub schema: String,
    /// Stable identifier for this emergence.
    pub emergence_id: String,
    /// The canonical code that was violated, when one exists.
    ///
    /// `None` means the catalog has no contract for this emergence yet — an
    /// explicit unknown, and itself a finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub violated_code: Option<CynicErrorCode>,
    /// The layer that owns the violated contract.
    pub owning_layer: ContractLayer,
    /// What the system was expected to do.
    pub expected: String,
    /// What was actually observed.
    pub observed: String,
    /// Evidence digests that prove the observation.
    #[serde(default)]
    pub evidence_digests: Vec<String>,
    pub repair: RepairStatus,
}

/// Validate an entry against the contracts it claims to record.
///
/// Returns the violations rather than a bool: a caller that persists a
/// malformed emergence record degrades the gallery into prose.
pub fn validate_gallery_entry(entry: &EmergenceGalleryEntry) -> Vec<&'static str> {
    let mut violations = Vec::new();
    if entry.schema != EMERGENCE_GALLERY_ENTRY_SCHEMA {
        violations.push("emergence_gallery_schema_invalid");
    }
    if entry.emergence_id.trim().is_empty() {
        violations.push("emergence_gallery_id_missing");
    }
    if entry.expected.trim().is_empty() {
        violations.push("emergence_gallery_expected_missing");
    }
    if entry.observed.trim().is_empty() {
        violations.push("emergence_gallery_observed_missing");
    }
    // An observation without evidence is a story, not a record.
    if entry.evidence_digests.is_empty() {
        violations.push("emergence_gallery_evidence_missing");
    }
    match &entry.repair {
        RepairStatus::Repaired {
            regression_proof, ..
        } => {
            if regression_proof.trim().is_empty() {
                violations.push("emergence_gallery_regression_proof_missing");
            }
        }
        RepairStatus::Diagnosed { root_cause } => {
            if root_cause.trim().is_empty() {
                violations.push("emergence_gallery_root_cause_missing");
            }
        }
        RepairStatus::Contained {
            residual_risk,
            removal_condition,
        } => {
            if residual_risk.trim().is_empty() {
                violations.push("emergence_gallery_residual_risk_missing");
            }
            if removal_condition.trim().is_empty() {
                violations.push("emergence_gallery_removal_condition_missing");
            }
        }
        RepairStatus::Observed => {}
    }
    violations
}

/// Persist an entry as a content-addressed evidence envelope.
///
/// Reuses the canonical evidence envelope instead of inventing a second
/// storage shape. A malformed entry is rejected before it can be addressed,
/// so an invalid record never acquires a digest that makes it look proven.
pub fn record_emergence(
    entry: &EmergenceGalleryEntry,
    producer: &str,
    observed_at_unix_ms: u128,
    provenance: Vec<String>,
) -> Result<EvidenceEnvelope, String> {
    let violations = validate_gallery_entry(entry);
    if !violations.is_empty() {
        return Err(format!("invalid_gallery_entry: {}", violations.join(",")));
    }
    build_evidence_envelope(
        &format!("emergence://{}", entry.emergence_id),
        producer,
        observed_at_unix_ms,
        provenance,
        entry,
    )
}

/// Residual risk summary over a gallery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryResidualRisk {
    pub total: usize,
    pub open: usize,
    /// Open emergences at layers that can present failure as success.
    pub convergence_fabricating_open: usize,
    /// Emergences with no canonical code — contracts missing from the catalog.
    pub uncatalogued: usize,
    /// Containments whose removal condition is still outstanding.
    pub active_containments: usize,
}

/// Summarize residual risk across recorded emergences.
///
/// Deliberately not a score: a single number would hide that one open proof
/// defect matters more than several visible reconciliation failures.
pub fn summarize_residual_risk(entries: &[EmergenceGalleryEntry]) -> GalleryResidualRisk {
    let mut risk = GalleryResidualRisk {
        total: entries.len(),
        open: 0,
        convergence_fabricating_open: 0,
        uncatalogued: 0,
        active_containments: 0,
    };
    for entry in entries {
        if entry.repair.is_open() {
            risk.open += 1;
            if entry.owning_layer.can_fabricate_convergence() {
                risk.convergence_fabricating_open += 1;
            }
        }
        if entry.violated_code.is_none() {
            risk.uncatalogued += 1;
        }
        if matches!(entry.repair, RepairStatus::Contained { .. }) {
            risk.active_containments += 1;
        }
    }
    risk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repaired_entry() -> EmergenceGalleryEntry {
        EmergenceGalleryEntry {
            schema: EMERGENCE_GALLERY_ENTRY_SCHEMA.to_string(),
            emergence_id: "activation-readback-substring".to_string(),
            violated_code: Some(CynicErrorCode::ActivationReadBackAbsent),
            owning_layer: ContractLayer::Proof,
            expected: "inactive read-back must not prove activation".to_string(),
            observed: "contains(\"active\") matched \"inactive\"".to_string(),
            evidence_digests: vec!["sha256:deadbeef".to_string()],
            repair: RepairStatus::Repaired {
                root_cause: "substring match on read-back output".to_string(),
                regression_proof: "cynic-protocol node_activation::tests::inactive_read_back_is_not_activation_proof"
                    .to_string(),
                repaired_at: ContractLayer::Proof,
            },
        }
    }

    #[test]
    fn valid_repaired_entry_has_no_violations() {
        assert!(validate_gallery_entry(&repaired_entry()).is_empty());
    }

    /// A repair claim without a regression proof is a memory, not a repair.
    #[test]
    fn repaired_without_regression_proof_is_invalid() {
        let mut entry = repaired_entry();
        entry.repair = RepairStatus::Repaired {
            root_cause: "fixed it".to_string(),
            regression_proof: "   ".to_string(),
            repaired_at: ContractLayer::Proof,
        };
        assert!(
            validate_gallery_entry(&entry).contains(&"emergence_gallery_regression_proof_missing")
        );
    }

    /// An observation with no evidence digest is prose.
    #[test]
    fn entry_without_evidence_is_invalid() {
        let mut entry = repaired_entry();
        entry.evidence_digests.clear();
        assert!(validate_gallery_entry(&entry).contains(&"emergence_gallery_evidence_missing"));
    }

    #[test]
    fn containment_requires_residual_risk_and_removal_condition() {
        let mut entry = repaired_entry();
        entry.repair = RepairStatus::Contained {
            residual_risk: String::new(),
            removal_condition: String::new(),
        };
        let violations = validate_gallery_entry(&entry);
        assert!(violations.contains(&"emergence_gallery_residual_risk_missing"));
        assert!(violations.contains(&"emergence_gallery_removal_condition_missing"));
    }

    #[test]
    fn proof_and_observation_layers_can_fabricate_convergence() {
        assert!(ContractLayer::Proof.can_fabricate_convergence());
        assert!(ContractLayer::Observation.can_fabricate_convergence());
        assert!(!ContractLayer::Reconciliation.can_fabricate_convergence());
        assert!(!ContractLayer::ExecutionBoundary.can_fabricate_convergence());
        assert!(!ContractLayer::DesiredState.can_fabricate_convergence());
    }

    #[test]
    fn repaired_is_not_open_others_are() {
        assert!(!repaired_entry().repair.is_open());
        assert!(RepairStatus::Observed.is_open());
        assert!(
            RepairStatus::Diagnosed {
                root_cause: "x".to_string()
            }
            .is_open()
        );
        assert!(
            RepairStatus::Contained {
                residual_risk: "x".to_string(),
                removal_condition: "y".to_string()
            }
            .is_open()
        );
    }

    /// An invalid entry must never acquire a digest: a content-addressed
    /// malformed record looks proven while proving nothing.
    #[test]
    fn invalid_entry_is_not_recorded() {
        let mut entry = repaired_entry();
        entry.evidence_digests.clear();
        let result = record_emergence(&entry, "test-agent", 1_724_000_000_000, vec!["test".into()]);
        assert!(result.is_err(), "invalid entry must not be addressable");
    }

    #[test]
    fn valid_entry_records_as_evidence_envelope() {
        let entry = repaired_entry();
        let envelope = record_emergence(
            &entry,
            "test-agent",
            1_724_000_000_000,
            vec!["node_activation_tests".into()],
        )
        .expect("record");
        assert_eq!(
            envelope.subject,
            "emergence://activation-readback-substring"
        );
        assert!(!envelope.digest.is_empty());
        assert!(!envelope.payload_digest.is_empty());
    }

    /// The same emergence recorded twice yields the same payload digest:
    /// content-addressing means a re-observation is not a new finding.
    #[test]
    fn identical_entries_share_payload_digest() {
        let entry = repaired_entry();
        let first = record_emergence(&entry, "agent-a", 1_000, vec!["p".into()]).expect("first");
        let second = record_emergence(&entry, "agent-b", 2_000, vec!["p".into()]).expect("second");
        assert_eq!(first.payload_digest, second.payload_digest);
        // Different producer and timestamp still yield distinct envelopes.
        assert_ne!(first.digest, second.digest);
    }

    #[test]
    fn uncatalogued_emergence_is_counted_as_residual_risk() {
        let mut entry = repaired_entry();
        entry.violated_code = None;
        let risk = summarize_residual_risk(&[entry]);
        assert_eq!(risk.uncatalogued, 1);
    }

    /// One open proof defect must stay visible next to louder but safer
    /// failures — this is why the summary is not collapsed into a score.
    #[test]
    fn open_proof_defect_is_counted_separately() {
        let entries = vec![
            EmergenceGalleryEntry {
                repair: RepairStatus::Observed,
                owning_layer: ContractLayer::Proof,
                ..repaired_entry()
            },
            EmergenceGalleryEntry {
                emergence_id: "recon-1".to_string(),
                repair: RepairStatus::Observed,
                owning_layer: ContractLayer::Reconciliation,
                ..repaired_entry()
            },
            EmergenceGalleryEntry {
                emergence_id: "exec-1".to_string(),
                repair: RepairStatus::Observed,
                owning_layer: ContractLayer::ExecutionBoundary,
                ..repaired_entry()
            },
        ];
        let risk = summarize_residual_risk(&entries);
        assert_eq!(risk.total, 3);
        assert_eq!(risk.open, 3);
        assert_eq!(risk.convergence_fabricating_open, 1);
    }

    #[test]
    fn containment_is_tracked_as_active() {
        let entry = EmergenceGalleryEntry {
            repair: RepairStatus::Contained {
                residual_risk: "guard in CLI only".to_string(),
                removal_condition: "port exposes typed status".to_string(),
            },
            ..repaired_entry()
        };
        let risk = summarize_residual_risk(&[entry]);
        assert_eq!(risk.active_containments, 1);
        assert_eq!(risk.open, 1);
    }

    #[test]
    fn empty_gallery_has_no_risk() {
        let risk = summarize_residual_risk(&[]);
        assert_eq!(risk.total, 0);
        assert_eq!(risk.open, 0);
        assert_eq!(risk.convergence_fabricating_open, 0);
    }

    #[test]
    fn entry_round_trips_through_json() {
        let entry = repaired_entry();
        let json = serde_json::to_string(&entry).expect("serialize");
        let back: EmergenceGalleryEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, entry);
    }
}
