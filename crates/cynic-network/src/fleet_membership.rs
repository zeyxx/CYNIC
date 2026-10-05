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
                    declared_state: DeclaredState::Planned,
                    ..
                }
            ) || !matches!(d, MembershipDifference::OrphanDeclaration { .. })
        })
    }
}

/// Parse a Proxmox `/api2/json/cluster/resources?type=vm` payload into
/// observed members.
///
/// Pure. The payload is the documented Proxmox cluster resources shape: a
/// `data` array whose VM entries carry `vmid`, `name` and a running state. The
/// hypervisor is the inventory of record, so the name it reports is the name the
/// comparison uses; inventing one would match the wrong declaration.
///
/// Entries carrying no usable name are skipped rather than defaulted: an
/// observation with an invented identity is worse than an absent observation.
/// An absent running state is an observation gap, so it is reported as not
/// running and the caller sees the gap rather than a fabricated success.
pub fn parse_proxmox_cluster_resources(
    payload: &serde_json::Value,
) -> Result<Vec<ObservedMember>, String> {
    let data = payload
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "proxmox cluster resources payload has no data array".to_string())?;

    let mut members = Vec::new();
    for entry in data {
        let Some(name) = entry
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            continue;
        };
        let running = entry
            .get("status")
            .and_then(serde_json::Value::as_str)
            .or_else(|| entry.get("node").and_then(serde_json::Value::as_str))
            .is_some_and(|state| state.eq_ignore_ascii_case("running"));
        members.push(ObservedMember {
            name: name.to_string(),
            address: None,
            running,
        });
    }
    Ok(members)
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

    /// The naming defect the measurement found, stated exactly. `cynic-lxc-headscale`
    /// is declared and does not exist under that name anywhere; `organ-headscale`
    /// is observed and exists only under that name; both claim 10.10.10.40.
    ///
    /// The report carries TWO facts, not one and not three. It compares by name,
    /// so the two names never meet and no AddressDivergence is produced: pairing
    /// them by address would report nothing at all and hide the defect, and
    /// inventing a divergence between two nodes that share no name would
    /// fabricate a relationship the registry does not claim. Two facts, both true,
    /// and a reader has to notice that one address appears on both sides.
    #[test]
    fn a_naming_defect_is_reported_as_two_facts_never_paired_by_address() {
        let mut node = declared("cynic-lxc-headscale");
        node.address = Some("10.10.10.40".to_string());
        let mut seen = observed("organ-headscale", true);
        seen.address = Some("10.10.10.40".to_string());

        let report = compare_membership(&[node], &[seen], "proxmox:pct-list", 1);
        assert_eq!(report.orphan_declarations(), vec!["cynic-lxc-headscale"]);
        assert_eq!(report.unregistered_members(), vec!["organ-headscale"]);
        assert_eq!(
            report.differences,
            vec![
                MembershipDifference::UnregisteredMember {
                    name: "organ-headscale".to_string(),
                    observed_address: Some("10.10.10.40".to_string()),
                },
                MembershipDifference::OrphanDeclaration {
                    name: "cynic-lxc-headscale".to_string(),
                    declared_address: Some("10.10.10.40".to_string()),
                    declared_state: DeclaredState::Deployed,
                },
            ]
        );
    }

    /// When the same NAME exists on both sides, membership agrees even when the
    /// addresses do not, and the divergence is reported on its own.
    #[test]
    fn a_matching_name_with_a_divergent_address_is_only_a_divergence() {
        let mut node = declared("organ-headscale");
        node.address = Some("10.10.10.40".to_string());
        let mut seen = observed("organ-headscale", true);
        seen.address = Some("10.10.10.99".to_string());

        let report = compare_membership(&[node], &[seen], "proxmox:pct-list", 1);
        assert!(report.orphan_declarations().is_empty());
        assert!(report.unregistered_members().is_empty());
        assert_eq!(
            report.differences,
            vec![MembershipDifference::AddressDivergence {
                name: "organ-headscale".to_string(),
                declared: Some("10.10.10.40".to_string()),
                observed: Some("10.10.10.99".to_string()),
            }]
        );
    }

    /// The shared address is present in both reports and nothing correlates them.
    /// That is the reader's job, and it is a real one: 10.10.10.40 is the address
    /// the registry gives a node that does not exist and the address the
    /// infrastructure gives a node the registry does not name.
    #[test]
    fn the_same_address_can_appear_on_both_sides_without_being_correlated() {
        let mut node = declared("cynic-lxc-headscale");
        node.address = Some("10.10.10.40".to_string());
        let mut seen = observed("organ-headscale", true);
        seen.address = Some("10.10.10.40".to_string());

        let report = compare_membership(&[node], &[seen], "proxmox:pct-list", 1);
        let declared_addresses: Vec<_> = report
            .differences
            .iter()
            .filter_map(|d| match d {
                MembershipDifference::OrphanDeclaration {
                    declared_address, ..
                } => declared_address.clone(),
                _ => None,
            })
            .collect();
        let observed_addresses: Vec<_> = report
            .differences
            .iter()
            .filter_map(|d| match d {
                MembershipDifference::UnregisteredMember {
                    observed_address, ..
                } => observed_address.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(declared_addresses, observed_addresses);
        assert!(
            !report
                .differences
                .iter()
                .any(|d| matches!(d, MembershipDifference::AddressDivergence { .. })),
            "no divergence may be invented between nodes that share no name"
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

    /// The shape observed on the builder, transcribed: five containers, one of
    /// which the registry cannot name. The parser must report all five, so the
    /// comparison can report the one that is undeclared.
    #[test]
    fn proxmox_cluster_resources_yields_every_named_guest() {
        let payload = serde_json::json!({
            "data": [
                {"vmid": 100, "name": "organ-headscale", "status": "running", "node": "pve"},
                {"vmid": 101, "name": "organ-forgejo",   "status": "running", "node": "pve"},
                {"vmid": 102, "name": "cynic-agent-host","status": "running", "node": "pve"},
                {"vmid": 103, "name": "cynic-lxc-surrealdb", "status": "running", "node": "pve"},
                {"vmid": 105, "name": "cynic-builder",    "status": "running", "node": "pve"}
            ]
        });
        let observed = parse_proxmox_cluster_resources(&payload).expect("parse");
        assert_eq!(observed.len(), 5);
        assert!(observed.iter().all(|m| m.running));
        assert!(observed.iter().any(|m| m.name == "organ-headscale"));
    }

    /// A stopped guest is observed as stopped. Absence of a status field is a
    /// gap and is reported the same way, because the caller cannot tell a
    /// missing field from a stopped guest without the raw payload.
    #[test]
    fn a_stopped_or_unlabelled_guest_is_not_running() {
        let payload = serde_json::json!({
            "data": [
                {"vmid": 105, "name": "cynic-builder", "status": "stopped"},
                {"vmid": 106, "name": "cynic-spare"}
            ]
        });
        let observed = parse_proxmox_cluster_resources(&payload).expect("parse");
        assert!(observed.iter().all(|m| !m.running));
        assert_eq!(observed.len(), 2);
    }

    /// An entry with no name cannot be matched against any declaration. Skipping
    /// it loses one observation; defaulting its name would invent an identity and
    /// could match it against the wrong declaration.
    #[test]
    fn an_entry_without_a_name_is_skipped_rather_than_defaulted() {
        let payload = serde_json::json!({
            "data": [
                {"vmid": 107},
                {"vmid": 108, "name": "   "},
                {"vmid": 105, "name": "cynic-builder", "status": "running"}
            ]
        });
        let observed = parse_proxmox_cluster_resources(&payload).expect("parse");
        assert_eq!(
            observed.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
            vec!["cynic-builder"]
        );
    }

    #[test]
    fn a_payload_without_a_data_array_is_an_error_not_an_empty_fleet() {
        assert!(parse_proxmox_cluster_resources(&serde_json::json!({})).is_err());
        assert!(parse_proxmox_cluster_resources(&serde_json::json!({"data": "x"})).is_err());
    }

    /// The end-to-end shape O1 exists to produce: what the fleet says, against
    /// what the registry declares, with the coordinator reported as undeclared
    /// and the phantom coordinator reported as declared-and-absent.
    #[test]
    fn the_measured_shape_produces_both_defects_at_once() {
        let declared_fleet = [
            declared("cynic-host-proxmox"),
            declared("cynic-host-cortex"),
            declared("local-kernel"),
            declared("local-builder"),
            declared("cynic-lxc-surrealdb"),
            declared("cynic-builder"),
            declared("forgejo"),
            declared("cynic-lxc-headscale"),
        ];
        let observed = parse_proxmox_cluster_resources(&serde_json::json!({
            "data": [
                {"vmid": 100, "name": "organ-headscale", "status": "running"},
                {"vmid": 101, "name": "organ-forgejo", "status": "running"},
                {"vmid": 102, "name": "cynic-agent-host", "status": "running"},
                {"vmid": 103, "name": "cynic-lxc-surrealdb", "status": "running"},
                {"vmid": 105, "name": "cynic-builder", "status": "running"}
            ]
        }))
        .expect("parse");

        let report = compare_membership(&declared_fleet, &observed, "proxmox:cluster-resources", 1);
        assert_eq!(
            report.unregistered_members(),
            // Order follows the observed payload, not the payload's VMID sort.
            // The hypervisor is the inventory of record, so its order is the
            // order the differences are reported in.
            vec!["organ-headscale", "organ-forgejo", "cynic-agent-host"]
        );
        assert_eq!(
            report.orphan_declarations(),
            // Order follows the declared inventory, for the same reason.
            vec![
                "cynic-host-proxmox",
                "cynic-host-cortex",
                "local-kernel",
                "local-builder",
                "forgejo",
                "cynic-lxc-headscale",
            ]
        );
        assert!(!report.every_declared_node_was_observed());
    }
}
