//! Pure comparison of registry claims against observed substrate facts.
//!
//! The registry is desired state. It is never proof, and the cortex incident
//! showed what happens when that distinction is lost: the registry declared
//! `kernel_primary` at `10.10.10.100` while the kernel ran on cortex, and
//! `kernel.synapse.cynic` resolved three different ways depending on which node
//! asked. Nothing detected it, because nothing compared the two sides.
//!
//! This module owns the *decision* only. It performs no I/O: an adapter probes
//! the substrate and hands the observations in, so the same policy is testable
//! without a network, a registry file, or a reachable peer.
//!
//! Three states, never a boolean:
//!
//! - `Confirmed`   — the substrate agrees with the claim
//! - `Refuted`     — the substrate contradicts it (this is the finding)
//! - `Unobservable` — the probe could not reach a verdict; unknown is data, and
//!   reporting it as healthy is exactly how a report fabricates convergence

use serde::{Deserialize, Serialize};

use crate::error_catalog::CynicErrorCode;
use crate::health_signal::HealthSignalPayload;

pub const REGISTRY_CLAIM_AUDIT_SCHEMA: &str = "cynic.registry_claim_audit.v1";

/// Which registry assertion a claim is about.
///
/// Typed rather than a free string so the reconciler branches on data. A new
/// claim kind must be added here, which forces every consumer to consider it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    /// The node declared to carry a role actually carries it.
    RoleHost,
    /// The declared address answers.
    SynapseAddress,
    /// The declared name resolves, and resolves consistently.
    DnsRecord,
    /// A declared service is listening where the registry says.
    ServiceBinding,
    /// The declared address is *reserved*, not merely correct today.
    ///
    /// A DHCP lease that currently matches the registry is not a reservation:
    /// `cynic-builder` answers on its declared `.119` while its guest config says
    /// `ip=dhcp`, so every peer trusting that address depends on a lease nobody
    /// pinned. Reachability probes cannot see this — the address is right until
    /// the day it silently is not.
    SynapseAllocation,
    /// A node's overlay address is declared, not just live.
    ///
    /// The WireGuard mesh carries real traffic while all five registry peers
    /// declare `address: -`, so the registry cannot say which node holds which
    /// overlay address and no watchdog threshold can be attributed to a peer.
    OverlayAddress,
    /// An address on a canonical plane belongs to a node the registry models.
    ///
    /// `organ-headscale` holds `10.10.10.40` on the canonical service plane and
    /// appears nowhere in the registry. An unmodelled occupant cannot be
    /// governed, and its address cannot be protected from reallocation.
    PlaneOccupancy,
}

impl ClaimKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RoleHost => "role_host",
            Self::SynapseAddress => "synapse_address",
            Self::DnsRecord => "dns_record",
            Self::ServiceBinding => "service_binding",
            Self::SynapseAllocation => "synapse_allocation",
            Self::OverlayAddress => "overlay_address",
            Self::PlaneOccupancy => "plane_occupancy",
        }
    }
}

/// A registry assertion paired with what the substrate reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimObservation {
    pub node_id: String,
    pub kind: ClaimKind,
    /// What the registry declares.
    pub declared: String,
    /// What the probe saw. `None` means the probe reached no verdict — this is
    /// preserved as unobservable rather than collapsed into a refutation.
    pub observed: Option<String>,
    /// True when the registry marks the entry `status: candidate`, i.e. it
    /// already admits the claim is unproven.
    pub declared_as_candidate: bool,
    /// Why the probe could not decide, when `observed` is `None`.
    pub unobservable_reason: Option<String>,
}

/// The verdict for one claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum ClaimVerdict {
    Confirmed,
    Refuted {
        declared: String,
        observed: String,
        /// A candidate entry is stale intent, not a substrate fault.
        declared_as_candidate: bool,
    },
    Unobservable {
        reason: String,
    },
}

impl ClaimVerdict {
    /// Whether this verdict contradicts the registry.
    pub fn is_refuted(&self) -> bool {
        matches!(self, Self::Refuted { .. })
    }

    /// Whether the claim is still unproven in either direction.
    ///
    /// Unobservable counts as open: a claim nobody could check is not a claim
    /// that held.
    pub fn is_open(&self) -> bool {
        !matches!(self, Self::Confirmed)
    }
}

/// Decide one claim from its observation. Pure.
///
/// Comparison is exact string equality on purpose. A fuzzy match would let
/// `10.10.10.100` and `10.10.10.1` agree, and an address comparison that
/// tolerates near-misses is worse than none.
pub fn judge_claim(observation: &ClaimObservation) -> ClaimVerdict {
    match &observation.observed {
        None => ClaimVerdict::Unobservable {
            reason: observation
                .unobservable_reason
                .clone()
                .unwrap_or_else(|| "probe returned no verdict and gave no reason".to_string()),
        },
        Some(observed) if observed == &observation.declared => ClaimVerdict::Confirmed,
        Some(observed) => ClaimVerdict::Refuted {
            declared: observation.declared.clone(),
            observed: observed.clone(),
            declared_as_candidate: observation.declared_as_candidate,
        },
    }
}

/// Project a refuted claim into the canonical health signal payload.
///
/// Returns `None` for anything that is not a refutation: a confirmed or
/// unobservable claim is not a divergence, and emitting a signal for it would
/// make the fleet look broken whenever a probe simply could not run.
pub fn claim_to_signal_payload(
    observation: &ClaimObservation,
    verdict: &ClaimVerdict,
) -> Option<HealthSignalPayload> {
    match verdict {
        ClaimVerdict::Refuted {
            declared,
            observed,
            declared_as_candidate,
        } => Some(HealthSignalPayload::RegistrySubstrateDivergence {
            node_id: observation.node_id.clone(),
            claim_kind: observation.kind.as_str().to_string(),
            declared: declared.clone(),
            observed: observed.clone(),
            declared_as_candidate: *declared_as_candidate,
        }),
        _ => None,
    }
}

/// Coverage summary over an audited set of claims.
///
/// `unobservable` is a first-class number, not an error count: per the metric
/// rule, 100% confirmation is not the target and residual unknown must stay
/// visible instead of being rounded away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimAuditSummary {
    pub schema: String,
    pub total: usize,
    pub confirmed: usize,
    pub refuted: usize,
    pub unobservable: usize,
    /// Refutations where the registry did NOT mark the entry candidate — one
    /// side is simply wrong.
    pub asserted_refutations: usize,
    /// Canonical codes raised by this audit, deduplicated.
    pub codes: Vec<CynicErrorCode>,
}

/// One name as answered from several vantage points.
///
/// Deliberately multi-observer: a node cannot detect this class from its own
/// resolver. Cortex answering `127.0.0.1` for `kernel.synapse.cynic` is locally
/// correct and globally wrong, and only comparing answers reveals it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameObservation {
    pub name: String,
    /// `(observer_id, resolved_value)` per vantage point that answered.
    pub answers: Vec<(String, String)>,
    /// `(observer_id, reason)` per vantage point that could not resolve.
    pub unresolved: Vec<(String, String)>,
}

/// Verdict for a name across observers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum NameVerdict {
    /// Every observer that answered gave the same value.
    Consistent { value: String, observers: usize },
    /// Observers disagree — the finding.
    Inconsistent { distinct_values: Vec<String> },
    /// Fewer than two observers answered, so consistency is undecidable.
    ///
    /// One answer is not agreement: it is a single opinion. Calling it
    /// consistent would let a one-node probe certify the whole fleet.
    Undecidable { reason: String },
}

/// Decide a name's consistency. Pure.
pub fn judge_name(observation: &NameObservation) -> NameVerdict {
    if observation.answers.len() < 2 {
        return NameVerdict::Undecidable {
            reason: format!(
                "{} of {} vantage point(s) resolved {}; at least 2 answers are \
                 needed before consistency means anything",
                observation.answers.len(),
                observation.answers.len() + observation.unresolved.len(),
                observation.name
            ),
        };
    }

    let distinct: std::collections::BTreeSet<&str> = observation
        .answers
        .iter()
        .map(|(_, value)| value.as_str())
        .collect();

    if distinct.len() == 1 {
        NameVerdict::Consistent {
            value: distinct
                .into_iter()
                .next()
                .unwrap_or_default()
                .to_string(),
            observers: observation.answers.len(),
        }
    } else {
        NameVerdict::Inconsistent {
            distinct_values: distinct.into_iter().map(str::to_string).collect(),
        }
    }
}

/// Project an inconsistent name into the canonical signal payload.
pub fn name_to_signal_payload(
    observation: &NameObservation,
    verdict: &NameVerdict,
) -> Option<HealthSignalPayload> {
    match verdict {
        NameVerdict::Inconsistent { .. } => {
            Some(HealthSignalPayload::NameResolutionInconsistent {
                name: observation.name.clone(),
                answers: observation.answers.clone(),
                unresolved_from: observation
                    .unresolved
                    .iter()
                    .map(|(observer, _)| observer.clone())
                    .collect(),
            })
        }
        _ => None,
    }
}

/// Summarize verdicts. Pure.
pub fn summarize_claims(verdicts: &[ClaimVerdict]) -> ClaimAuditSummary {
    let mut confirmed = 0;
    let mut refuted = 0;
    let mut unobservable = 0;
    let mut asserted_refutations = 0;

    for verdict in verdicts {
        match verdict {
            ClaimVerdict::Confirmed => confirmed += 1,
            ClaimVerdict::Unobservable { .. } => unobservable += 1,
            ClaimVerdict::Refuted {
                declared_as_candidate,
                ..
            } => {
                refuted += 1;
                if !declared_as_candidate {
                    asserted_refutations += 1;
                }
            }
        }
    }

    let mut codes = Vec::new();
    if refuted > 0 {
        codes.push(CynicErrorCode::FleetRegistrySubstrateDivergence);
    }

    ClaimAuditSummary {
        schema: REGISTRY_CLAIM_AUDIT_SCHEMA.to_string(),
        total: verdicts.len(),
        confirmed,
        refuted,
        unobservable,
        asserted_refutations,
        codes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(observed: Option<&str>, candidate: bool) -> ClaimObservation {
        ClaimObservation {
            node_id: "local-kernel".to_string(),
            kind: ClaimKind::RoleHost,
            declared: "10.10.10.100".to_string(),
            observed: observed.map(str::to_string),
            declared_as_candidate: candidate,
            unobservable_reason: None,
        }
    }

    /// The live cortex case: the registry names the kernel at 10.10.10.100, the
    /// substrate serves it from cortex. That contradiction is the finding.
    #[test]
    fn a_contradicted_claim_is_refuted_and_becomes_a_signal() {
        let observation = claim(Some("192.168.100.2"), true);
        let verdict = judge_claim(&observation);

        assert!(verdict.is_refuted());
        assert!(verdict.is_open());

        let payload = claim_to_signal_payload(&observation, &verdict)
            .expect("a refutation must project to a signal");
        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::FleetRegistrySubstrateDivergence]
        );
    }

    /// A probe that reached no verdict must never be read as agreement.
    ///
    /// This is the failure mode that let a stale claim survive for weeks: an
    /// unchecked claim looked identical to a verified one. Unobservable stays
    /// open and emits no divergence signal, so the fleet is not reported broken
    /// merely because a probe could not run.
    #[test]
    fn an_unobservable_claim_is_open_but_not_a_divergence() {
        let mut observation = claim(None, false);
        observation.unobservable_reason = Some("ICMP filtered on this path".to_string());

        let verdict = judge_claim(&observation);
        match &verdict {
            ClaimVerdict::Unobservable { reason } => assert!(reason.contains("ICMP")),
            other => panic!("expected Unobservable, got {other:?}"),
        }
        assert!(verdict.is_open(), "unchecked is not proven");
        assert!(!verdict.is_refuted(), "unchecked is not refuted either");
        assert!(claim_to_signal_payload(&observation, &verdict).is_none());
    }

    /// An unobservable verdict always carries a reason, even when the probe
    /// forgot to give one — a bare "unknown" hides why.
    #[test]
    fn unobservable_without_a_reason_still_explains_itself() {
        let verdict = judge_claim(&claim(None, false));
        match verdict {
            ClaimVerdict::Unobservable { reason } => assert!(!reason.trim().is_empty()),
            other => panic!("expected Unobservable, got {other:?}"),
        }
    }

    #[test]
    fn a_matching_claim_is_confirmed_and_closed() {
        let verdict = judge_claim(&claim(Some("10.10.10.100"), false));
        assert_eq!(verdict, ClaimVerdict::Confirmed);
        assert!(!verdict.is_open());
    }

    /// Comparison is exact: a near-miss address is a refutation, not a match.
    #[test]
    fn a_near_miss_address_is_refuted_not_tolerated() {
        assert!(judge_claim(&claim(Some("10.10.10.1"), false)).is_refuted());
    }

    /// The summary keeps unobservable visible and separates asserted
    /// refutations from unmaterialized candidates.
    #[test]
    fn summary_reports_residual_unknown_and_asserted_refutations() {
        let verdicts = vec![
            judge_claim(&claim(Some("10.10.10.100"), false)),
            judge_claim(&claim(Some("192.168.100.2"), true)),
            judge_claim(&claim(Some("192.168.100.2"), false)),
            judge_claim(&claim(None, false)),
        ];
        let summary = summarize_claims(&verdicts);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.confirmed, 1);
        assert_eq!(summary.refuted, 2);
        assert_eq!(summary.unobservable, 1);
        assert_eq!(
            summary.asserted_refutations, 1,
            "only the non-candidate refutation is an asserted falsehood"
        );
        assert_eq!(
            summary.codes,
            vec![CynicErrorCode::FleetRegistrySubstrateDivergence]
        );
    }

    /// A clean audit raises no codes.
    #[test]
    fn a_fully_confirmed_audit_raises_no_codes() {
        let summary = summarize_claims(&[judge_claim(&claim(Some("10.10.10.100"), false))]);
        assert!(summary.codes.is_empty());
        assert_eq!(summary.refuted, 0);
    }

    /// The live name case: three vantage points, three answers.
    #[test]
    fn disagreeing_observers_make_a_name_inconsistent() {
        let observation = NameObservation {
            name: "kernel.synapse.cynic".to_string(),
            answers: vec![
                ("cortex".to_string(), "127.0.0.1".to_string()),
                ("cynic-builder".to_string(), "192.168.100.2".to_string()),
                ("registry".to_string(), "10.10.10.100".to_string()),
            ],
            unresolved: vec![("forgejo".to_string(), "NXDOMAIN".to_string())],
        };
        let verdict = judge_name(&observation);

        match &verdict {
            NameVerdict::Inconsistent { distinct_values } => assert_eq!(distinct_values.len(), 3),
            other => panic!("expected Inconsistent, got {other:?}"),
        }

        let payload = name_to_signal_payload(&observation, &verdict).expect("signal");
        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::FleetNameResolutionInconsistent]
        );
        // The vantage point that could not resolve is preserved as unknown, not
        // counted among those that agreed.
        match payload {
            HealthSignalPayload::NameResolutionInconsistent {
                unresolved_from, ..
            } => assert_eq!(unresolved_from, vec!["forgejo".to_string()]),
            other => panic!("unexpected payload {other:?}"),
        }
    }

    /// A single answer is one opinion, never fleet-wide agreement.
    ///
    /// Without this, a one-node probe could certify a name the rest of the fleet
    /// resolves differently — the exact blind spot that hid the cortex case.
    #[test]
    fn one_answer_cannot_certify_a_name() {
        let verdict = judge_name(&NameObservation {
            name: "kernel.synapse.cynic".to_string(),
            answers: vec![("cortex".to_string(), "127.0.0.1".to_string())],
            unresolved: vec![("cynic-builder".to_string(), "timeout".to_string())],
        });
        match verdict {
            NameVerdict::Undecidable { reason } => assert!(reason.contains("at least 2")),
            other => panic!("expected Undecidable, got {other:?}"),
        }
    }

    #[test]
    fn agreeing_observers_make_a_name_consistent() {
        let verdict = judge_name(&NameObservation {
            name: "cynic-builder.synapse.cynic".to_string(),
            answers: vec![
                ("cortex".to_string(), "10.10.10.119".to_string()),
                ("forgejo".to_string(), "10.10.10.119".to_string()),
            ],
            unresolved: vec![],
        });
        assert_eq!(
            verdict,
            NameVerdict::Consistent {
                value: "10.10.10.119".to_string(),
                observers: 2,
            }
        );
    }

    /// Claim kinds project to the strings the reconciler branches on; a rename
    /// that broke this would silently change the signal contract.
    /// Superseded by `all_claim_kinds_have_stable_wire_names`, which covers the
    /// same contract for every variant including the three added for the
    /// network/federation audit.
    #[test]
    fn claim_kinds_have_stable_wire_names() {
        assert_eq!(ClaimKind::RoleHost.as_str(), "role_host");
    }

    /// F2b: an address that is correct today but only leased.
    ///
    /// cynic-builder answers on its declared 10.10.10.119 while its Proxmox
    /// config says `ip=dhcp`. A reachability probe confirms the address and
    /// learns nothing — the claim that matters is whether it is RESERVED.
    /// Declared "static" versus observed "dhcp" must refute.
    #[test]
    fn a_leased_address_refutes_a_static_allocation_claim() {
        let observation = ClaimObservation {
            node_id: "cynic-builder".to_string(),
            kind: ClaimKind::SynapseAllocation,
            declared: "static:10.10.10.119".to_string(),
            observed: Some("dhcp:10.10.10.119".to_string()),
            declared_as_candidate: true,
            unobservable_reason: None,
        };
        let verdict = judge_claim(&observation);
        assert!(
            verdict.is_refuted(),
            "same address, different tenure: a lease is not a reservation"
        );
        assert!(claim_to_signal_payload(&observation, &verdict).is_some());
    }

    /// A genuinely reserved address confirms.
    #[test]
    fn a_reserved_address_confirms_the_allocation_claim() {
        let observation = ClaimObservation {
            node_id: "cynic-lxc-surrealdb".to_string(),
            kind: ClaimKind::SynapseAllocation,
            declared: "static:10.10.10.103".to_string(),
            observed: Some("static:10.10.10.103".to_string()),
            declared_as_candidate: false,
            unobservable_reason: None,
        };
        assert_eq!(judge_claim(&observation), ClaimVerdict::Confirmed);
    }

    /// F5: the overlay carries traffic while the registry declares no address.
    ///
    /// All five WireGuard peers declare `address: -` yet 10.20.0.1/2/4/5 answer.
    /// An undeclared-but-live address is a refutation: the registry cannot
    /// attribute a watchdog threshold to a peer it cannot name.
    #[test]
    fn a_live_but_undeclared_overlay_address_is_refuted() {
        let observation = ClaimObservation {
            node_id: "cynic-host-cortex".to_string(),
            kind: ClaimKind::OverlayAddress,
            declared: "unset".to_string(),
            observed: Some("10.20.0.3".to_string()),
            declared_as_candidate: false,
            unobservable_reason: None,
        };
        let verdict = judge_claim(&observation);
        assert!(verdict.is_refuted());
        match &verdict {
            ClaimVerdict::Refuted {
                declared_as_candidate,
                ..
            } => assert!(
                !declared_as_candidate,
                "an unset overlay address is an asserted gap, not staged intent"
            ),
            other => panic!("expected Refuted, got {other:?}"),
        }
    }

    /// F2c: an occupant the registry does not model.
    ///
    /// organ-headscale holds 10.10.10.40 on the canonical service plane and is
    /// absent from `nodes[]`. Expressed as: the plane's occupant should be a
    /// modelled node, observed as an unmodelled one.
    #[test]
    fn an_unmodelled_plane_occupant_is_refuted() {
        let observation = ClaimObservation {
            node_id: "10.10.10.40".to_string(),
            kind: ClaimKind::PlaneOccupancy,
            declared: "unallocated".to_string(),
            observed: Some("organ-headscale (not in registry nodes)".to_string()),
            declared_as_candidate: false,
            unobservable_reason: None,
        };
        let verdict = judge_claim(&observation);
        assert!(
            verdict.is_refuted(),
            "an address the registry thinks is free but something holds must refute"
        );
        let payload = claim_to_signal_payload(&observation, &verdict).expect("signal");
        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::FleetRegistrySubstrateDivergence]
        );
    }

    /// Every claim kind projects a stable wire name.
    ///
    /// These strings cross the signal boundary, so a rename would silently
    /// change the contract the reconciler branches on.
    #[test]
    fn all_claim_kinds_have_stable_wire_names() {
        for (kind, expected) in [
            (ClaimKind::RoleHost, "role_host"),
            (ClaimKind::SynapseAddress, "synapse_address"),
            (ClaimKind::DnsRecord, "dns_record"),
            (ClaimKind::ServiceBinding, "service_binding"),
            (ClaimKind::SynapseAllocation, "synapse_allocation"),
            (ClaimKind::OverlayAddress, "overlay_address"),
            (ClaimKind::PlaneOccupancy, "plane_occupancy"),
        ] {
            assert_eq!(kind.as_str(), expected);
        }
    }
}
