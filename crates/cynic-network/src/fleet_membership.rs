//! Fleet membership comparison — pure, Zero-I/O.
//!
//! `contracts/infrastructure-discovery.json` INF1 and INF2 split the discovery of
//! fleet membership from the observation of it. This module is the observation
//! half: it takes what was declared and what was observed and reports the
//! difference. It performs no I/O, so it can be tested against any fleet shape.
//!
//! ## The rule this enforces
//!
//! A registry declares intent and establishes nothing. Only observation
//! establishes a fact about the running system
//! (`contracts/observability-facts.json` N5). The forbidden inference is
//! asymmetric, and this module is written so that neither side can be mistaken
//! for the other:
//!
//! - a member observed but not declared is an `unregistered_member`: the fleet
//!   is larger than the registry says;
//! - a member declared but not observed is an `orphan_declaration`: the
//!   registry describes something the observation did not see.
//!
//! An `orphan_declaration` is NOT proof of absence. A node may be planned and not
//! yet deployed, which is how deployment is expressed. The caller classifies a
//! node as planned, deployed or orphaned; this module only reports the difference
//! and never picks a winner.
//!
//! ## What was measured when this was written
//!
//! On the builder, `pct list` reported five containers: organ-headscale (100),
//! organ-forgejo (101), cynic-agent-host (102), cynic-lxc-surrealdb (103) and
//! cynic-builder (105). The repository registry declared eight nodes, two of
//! which resolve nowhere, and omitted three of those containers entirely.
//! `organ-headscale` runs headscale on port 8080 at the address the registry
//! assigns to a differently named node, and that real name appears in neither
//! registry nor any hosts file.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Schema for a fleet membership comparison report.
pub const FLEET_MEMBERSHIP_REPORT_SCHEMA: &str = "cynic.fleet.membership_report.v1";

/// Declared lifecycle state of a node. A declaration carries this so that a
/// planned node is distinguishable from a claim that it exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredState {
    /// Intended to exist and believed present.
    Deployed,
    /// Intended to exist and known not to be running yet.
    Planned,
}

/// A node as the registry declares it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredMember {
    pub name: String,
    /// Declared lifecycle state. Defaults to deployed when the registry carries
    /// no field, because a registry that lists a node without qualifying it is
    /// claiming it exists.
    #[serde(default)]
    pub state: DeclaredStateWrapper,
    /// Address the registry assigns, when it assigns one.
    #[serde(default)]
    pub address: Option<String>,
}

/// Newtype so `#[serde(default)]` yields the deployed default without a manual impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredStateWrapper(pub DeclaredState);

impl Default for DeclaredStateWrapper {
    fn default() -> Self {
        Self(DeclaredState::Deployed)
    }
}

/// A node as the infrastructure reported it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedMember {
    pub name: String,
    /// Address the infrastructure reports, when it reports one. A DHCP guest may
    /// report none, which is an observation gap rather than a missing node.
    #[serde(default)]
    pub address: Option<String>,
    /// Whether the infrastructure reports it running.
    pub running: bool,
}

/// One difference between the declared and the observed fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "class", rename_all = "snake_case")]
pub enum MembershipDifference {
    /// Observed on the infrastructure, absent from the registry.
    UnregisteredMember {
        name: String,
        observed_address: Option<String>,
    },
    /// In the registry, not observed. Not proof of absence: a planned node is
    /// expected to be absent.
    OrphanDeclaration {
        name: String,
        declared_address: Option<String>,
        declared_state: DeclaredState,
    },
    /// Present on both sides, but the addresses disagree. Recorded even though
    /// both nodes exist, because a name that resolves elsewhere is a naming
    /// defect rather than a membership one.
    AddressDivergence {
        name: String,
        declared: Option<String>,
        observed: Option<String>,
    },
    /// Present on both sides, but the registry says deployed and the
    /// infrastructure says not running.
    NotRunning { name: String },
}

/// The result of comparing a declared fleet against an observed one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetMembershipReport {
    pub schema: String,
    pub declared_count: usize,
    pub observed_count: usize,
    pub differences: Vec<MembershipDifference>,
    /// Identity of the reader that produced the observation. A membership fact
    /// without a reader is not an observation.
    pub observed_by: String,
    pub observed_at_unix_ms: u64,
}

impl FleetMembershipReport {
    pub fn unregistered_members(&self) -> Vec<&str> {
        self.differences
            .iter()
            .filter_map(|d| match d {
                MembershipDifference::UnregisteredMember { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn orphan_declarations(&self) -> Vec<&str> {
        self.differences
            .iter()
            .filter_map(|d| match d {
                MembershipDifference::OrphanDeclaration { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// True when every declared node was observed. This is the condition INF1
    /// asks for, and it is deliberately weaker than convergence: a planned node
    /// that has not been deployed does not make it false.
    pub fn every_declared_node_was_observed(&self) -> bool {
        self.differences.iter().all(|d| {
            matches!(
                d,
                MembershipDifference::OrphanDeclaration {
                    state: DeclaredState::Planned,
                    ..
                }
            ) || !matches!(d, MembershipDifference::OrphanDeclaration { .. })
        })
    }
}

/// Compare a declared fleet against an observed one.
///
/// Pure. Returns every difference rather than a verdict, because the caller
/// holds the authority for what to do about each class, and because a
/// comparison that returns a single boolean hides which side disagreed.
pub fn compare_membership(
    declared: &[DeclaredMember],
    observed: &[ObservedMember],
    observed_by: impl Into<String>,
    observed_at_unix_ms: u64,
) -> FleetMembershipReport {
    let observed_by = observed_by.into();
    let observed_names: BTreeMap<&str, &ObservedMember> = observed
        .iter()
        .map(|member| (member.name.as_str(), member))
        .collect();
    let declared_names: BTreeMap<&str, &DeclaredMember> = declared
        .iter()
        .map(|member| (member.name.as_str(), member))
        .collect();

    let mut differences = Vec::new();

    for member in observed {
        match declared_names.get(member.name.as_str()) {
            None => differences.push(MembershipDifference::UnregisteredMember {
                name: member.name.clone(),
                observed_address: member.address.clone(),
            }),
            Some(declared) => {
                if declared.address != member.address {
                    differences.push(MembershipDifference::AddressDivergence {
                        name: member.name.clone(),
                        declared: declared.address.clone(),
                        observed: member.address.clone(),
                    });
                }
                if !member.running && declared.state.0 == DeclaredState::Deployed {
                    differences.push(MembershipDifference::NotRunning {
                        name: member.name.clone(),
                    });
                }
            }
        }
    }

    for member in declared {
        if !observed_names.contains_key(member.name.as_str()) {
            differences.push(MembershipDifference::OrphanDeclaration {
                name: member.name.clone(),
                declared_address: member.address.clone(),
                declared_state: member.state.0,
            });
        }
    }

    FleetMembershipReport {
        schema: FLEET_MEMBERSHIP_REPORT_SCHEMA.to_string(),
        declared_count: declared.len(),
        observed_count: observed.len(),
        differences,
        observed_by,
        observed_at_unix_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(name: &str) -> DeclaredMember {
        DeclaredMember {
            name: name.to_string(),
            state: DeclaredStateWrapper::default(),
            address: None,
        }
    }

    fn observed(name: &str, running: bool) -> ObservedMember {
        ObservedMember {
            name: name.to_string(),
            address: None,
            running,
        }
    }

    /// The measurement that motivated this module: the fleet is larger than the
    /// registry, and the coordinator the fleet runs is one the registry cannot
    /// name. A comparison that only looked at declarations would miss both.
    #[test]
    fn the_live_fleet_is_larger_than_the_registry_declares() {
        let report = compare_membership(
            &[declared("cynic-builder"), declared("cynic-lxc-surrealdb")],
            &[
                observed("organ-headscale", true),
                observed("cynic-builder", true),
                observed("cynic-lxc-surrealdb", true),
            ],
            "proxmox:pct-list",
            1,
        );
        assert_eq!(report.unregistered_members(), vec!["organ-headscale"]);
        assert!(report.orphan_declarations().is_empty());
    }

    #[test]
    fn a_declared_node_that_resolves_nowhere_is_an_orphan_declaration_not_a_fault() {
        let report = compare_membership(
            &[declared("cynic-lxc-headscale")],
            &[],
            "proxmox:pct-list",
            1,
        );
        assert_eq!(report.orphan_declarations(), vec!["cynic-lxc-headscale"]);
        assert!(
            !report.every_declared_node_was_observed(),
            "a declared node that was not observed has not been converged, whatever the reason"
        );
    }

    /// The asymmetry that N5 forbids in prose. A planned node is expected to be
    /// absent, so its absence must not read as a defect.
    #[test]
    fn a_planned_node_being_absent_is_not_a_failure() {
        let mut planned = declared("organ-forgejo");
        planned.state = DeclaredStateWrapper(DeclaredState::Planned);
        let report = compare_membership(&[planned], &[], "proxmox:pct-list", 1);
        assert_eq!(report.orphan_declarations(), vec!["organ-forgejo"]);
        assert!(
            report.every_declared_node_was_observed(),
            "a planned node that has not been deployed is expected to be absent"
        );
    }

    /// The naming defect the measurement found: both names exist, the address
    /// does not agree, and neither side is wrong about its own existence.
    #[test]
    fn a_node_present_on_both_sides_with_a_divergent_address_is_reported() {
        let mut node = declared("cynic-lxc-headscale");
        node.address = Some("10.10.10.40".to_string());
        let mut seen = observed("organ-headscale", true);
        seen.address = Some("10.10.10.40".to_string());

        let report = compare_membership(&[node], &[seen], "proxmox:pct-list", 1);
        assert!(report.unregistered_members().is_empty());
        assert!(report.orphan_declarations().is_empty());
        assert_eq!(
            report.differences,
            vec![
                MembershipDifference::AddressDivergence {
                    name: "cynic-lxc-headscale".to_string(),
                    declared: Some("10.10.10.40".to_string()),
                    observed: Some("10.10.10.40".to_string()),
                },
                MembershipDifference::UnregisteredMember {
                    name: "organ-headscale".to_string(),
                    observed_address: Some("10.10.10.40".to_string()),
                },
            ]
        );
    }

    #[test]
    fn a_declared_deployed_node_that_is_not_running_is_reported() {
        let report = compare_membership(
            &[declared("cynic-builder")],
            &[observed("cynic-builder", false)],
            "proxmox:pct-list",
            1,
        );
        assert_eq!(
            report.differences,
            vec![MembershipDifference::NotRunning {
                name: "cynic-builder".to_string()
            }]
        );
    }

    #[test]
    fn an_observation_without_a_reader_is_still_carried_as_unknown() {
        let report = compare_membership(&[], &[observed("x", true)], "", 1);
        assert!(
            report.observed_by.is_empty(),
            "the reader is carried verbatim; an empty one is the caller's error to make visible"
        );
    }

    #[test]
    fn identical_fleets_produce_no_differences() {
        let mut node = declared("cynic-builder");
        node.address = Some("10.10.10.119".to_string());
        let mut seen = observed("cynic-builder", true);
        seen.address = Some("10.10.10.119".to_string());
        let report = compare_membership(&[node], &[seen], "proxmox:pct-list", 1);
        assert!(report.differences.is_empty());
        assert!(report.every_declared_node_was_observed());
    }
}
