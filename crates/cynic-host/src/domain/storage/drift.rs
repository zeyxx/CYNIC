//! Guest persistence drift — pure registry contract.
//!
//! A guest declared as a deployment target but lacking a boot-persistence
//! flag disappears silently on every host reboot: the provider stops it with
//! the rest of the `stopall` and never relaunches it, while the registry
//! keeps asserting the service is desired. This module turns that silent
//! divergence into an explicit, testable predicate.
//!
//! Observed instance (2026-10-07, proxmox-forge): CT 102 `cynic-agent-host`
//! ran for roughly five weeks, was stopped cleanly by the host reboot on
//! 2026-10-06, and was never relaunched because its config carried no
//! `onboot` key while every sibling guest declared `onboot=1`.
//!
//! Layering: cynic-lifecycle (domain). Pure: no I/O, no `std::process`,
//! no provider calls. Consumes registry-shaped data only; the port layer
//! supplies the observed state.

use serde::{Deserialize, Serialize};

/// Declared boot-persistence for one guest.
///
/// `Absent` is a distinct case from `Disabled` and matters: most providers
/// default an undeclared flag to off, so an absent key silently means "do
/// not start me", which is the exact failure this predicate exists to catch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredPersistence {
    /// The provider config declares the flag (value may still be false).
    Declared(bool),
    /// The provider config has no such key. Usually means default-off.
    Absent,
}

/// Observed persistence for one guest, read back from the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservedPersistence {
    /// Guest is configured to be started by the provider at host boot.
    BootPersistent,
    /// Guest is not configured to be started at host boot.
    NotBootPersistent,
    /// The provider did not expose this information for this guest.
    Unobserved,
}

/// A guest as declared by the registry, reduced to the persistence concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredGuest {
    pub node_id: String,
    /// True when the registry marks this node as a deployment target.
    pub deploy_target: bool,
    pub declared_persistence: DeclaredPersistence,
    /// True when the node is a hosted guest (has `proxmox_id`) rather than
    /// the baremetal host itself. Baremetal hosts are governed by the
    /// provider's own onboot configuration, not by a guest flag.
    pub hosted: bool,
}

/// A guest as observed from the provider, reduced to the persistence concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedGuest {
    pub node_id: String,
    pub observed_persistence: ObservedPersistence,
}

/// Why a guest's persistence does not match its declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceDriftKind {
    /// Declared deploy target, but the provider reports it will not start
    /// it at boot. This is the silent-disappearance failure.
    DeployTargetNotBootPersistent,
    /// The registry says nothing about persistence and the provider says
    /// nothing either: unresolved, not necessarily wrong.
    Unobserved,
}

/// One concrete divergence between declared and observed persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistenceDrift {
    pub node_id: String,
    pub kind: PersistenceDriftKind,
    /// Human-readable evidence, one line, no speculation.
    pub evidence: String,
}

/// Evaluate persistence drift across the whole fleet.
///
/// Pure: same inputs always yield the same output, no clock, no I/O.
/// Baremetal hosts are skipped: their boot behaviour is the provider
/// kernel's own concern, not a guest flag this registry governs.
pub fn evaluate_persistence_drift(
    declared: &[DeclaredGuest],
    observed: &[ObservedGuest],
) -> Vec<PersistenceDrift> {
    let mut drifts = Vec::new();

    for guest in declared {
        if !guest.deploy_target || !guest.hosted {
            continue;
        }

        let observed_entry = observed.iter().find(|o| o.node_id == guest.node_id);
        let observed_persistence = observed_entry
            .map(|o| o.observed_persistence)
            .unwrap_or(ObservedPersistence::Unobserved);

        match observed_persistence {
            ObservedPersistence::NotBootPersistent => {
                drifts.push(PersistenceDrift {
                    node_id: guest.node_id.clone(),
                    kind: PersistenceDriftKind::DeployTargetNotBootPersistent,
                    evidence: match guest.declared_persistence {
                        DeclaredPersistence::Absent => {
                            "registry declares this a deploy target; provider config has no onboot key, which defaults to off".to_string()
                        }
                        DeclaredPersistence::Declared(true) => {
                            "registry and provider config both declare boot persistence, yet the provider reports the guest is not boot persistent".to_string()
                        }
                        DeclaredPersistence::Declared(false) => {
                            "provider config explicitly disables boot persistence while the registry declares this a deploy target".to_string()
                        }
                    },
                });
            }
            ObservedPersistence::Unobserved => {
                drifts.push(PersistenceDrift {
                    node_id: guest.node_id.clone(),
                    kind: PersistenceDriftKind::Unobserved,
                    evidence: match guest.declared_persistence {
                        DeclaredPersistence::Absent => {
                            "registry declares this a deploy target with no persistence declared, and the provider exposes no boot-persistence state".to_string()
                        }
                        _ => {
                            "registry declares boot persistence but the provider exposes no boot-persistence state to confirm it".to_string()
                        }
                    },
                });
            }
            ObservedPersistence::BootPersistent => {}
        }
    }

    drifts.sort_by(|a, b| a.node_id.cmp(&b.node_id));
    drifts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guest(
        node_id: &str,
        deploy_target: bool,
        persistence: DeclaredPersistence,
    ) -> DeclaredGuest {
        DeclaredGuest {
            node_id: node_id.to_string(),
            deploy_target,
            declared_persistence: persistence,
            hosted: true,
        }
    }

    #[test]
    fn deploy_target_without_onboot_is_flagged() {
        let declared = vec![guest("cynic-agent-host", true, DeclaredPersistence::Absent)];
        let observed = vec![ObservedGuest {
            node_id: "cynic-agent-host".to_string(),
            observed_persistence: ObservedPersistence::NotBootPersistent,
        }];

        let drifts = evaluate_persistence_drift(&declared, &observed);

        assert_eq!(drifts.len(), 1, "the observed CT 102 case must be flagged");
        assert_eq!(
            drifts[0].kind,
            PersistenceDriftKind::DeployTargetNotBootPersistent
        );
        assert!(
            drifts[0].evidence.contains("no onboot key"),
            "evidence must name the absent key, got: {}",
            drifts[0].evidence
        );
    }

    #[test]
    fn non_deploy_target_is_not_flagged() {
        let declared = vec![guest(
            "cynic-lxc-surrealdb",
            false,
            DeclaredPersistence::Absent,
        )];
        let observed = vec![ObservedGuest {
            node_id: "cynic-lxc-surrealdb".to_string(),
            observed_persistence: ObservedPersistence::NotBootPersistent,
        }];

        assert!(evaluate_persistence_drift(&declared, &observed).is_empty());
    }

    #[test]
    fn baremetal_host_is_not_flagged() {
        let declared = vec![DeclaredGuest {
            node_id: "cynic-host-proxmox".to_string(),
            deploy_target: true,
            declared_persistence: DeclaredPersistence::Absent,
            hosted: false,
        }];
        let observed = vec![ObservedGuest {
            node_id: "cynic-host-proxmox".to_string(),
            observed_persistence: ObservedPersistence::NotBootPersistent,
        }];

        assert!(evaluate_persistence_drift(&declared, &observed).is_empty());
    }

    #[test]
    fn boot_persistent_deploy_target_is_clean() {
        let declared = vec![guest(
            "cynic-builder",
            true,
            DeclaredPersistence::Declared(true),
        )];
        let observed = vec![ObservedGuest {
            node_id: "cynic-builder".to_string(),
            observed_persistence: ObservedPersistence::BootPersistent,
        }];

        assert!(evaluate_persistence_drift(&declared, &observed).is_empty());
    }

    #[test]
    fn missing_observation_is_unobserved_not_failure() {
        let declared = vec![guest(
            "cynic-lxc-git",
            true,
            DeclaredPersistence::Declared(true),
        )];

        let drifts = evaluate_persistence_drift(&declared, &[]);

        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].kind, PersistenceDriftKind::Unobserved);
    }

    #[test]
    fn absent_declaration_with_missing_observation_names_the_gap() {
        let declared = vec![guest("cynic-agent-host", true, DeclaredPersistence::Absent)];

        let drifts = evaluate_persistence_drift(&declared, &[]);

        assert_eq!(drifts[0].kind, PersistenceDriftKind::Unobserved);
        assert!(
            drifts[0].evidence.contains("no persistence declared"),
            "evidence must name the absent declaration, got: {}",
            drifts[0].evidence
        );
    }

    #[test]
    fn explicitly_disabled_config_is_named_in_evidence() {
        let declared = vec![guest(
            "cynic-builder",
            true,
            DeclaredPersistence::Declared(false),
        )];
        let observed = vec![ObservedGuest {
            node_id: "cynic-builder".to_string(),
            observed_persistence: ObservedPersistence::NotBootPersistent,
        }];

        let drifts = evaluate_persistence_drift(&declared, &observed);

        assert_eq!(drifts.len(), 1);
        assert!(
            drifts[0].evidence.contains("explicitly disables"),
            "evidence must distinguish an explicit disable from an absent key, got: {}",
            drifts[0].evidence
        );
    }

    #[test]
    fn drift_output_is_deterministic() {
        let declared = vec![
            guest("zeta", true, DeclaredPersistence::Absent),
            guest("alpha", true, DeclaredPersistence::Declared(false)),
        ];
        let observed = vec![
            ObservedGuest {
                node_id: "zeta".to_string(),
                observed_persistence: ObservedPersistence::NotBootPersistent,
            },
            ObservedGuest {
                node_id: "alpha".to_string(),
                observed_persistence: ObservedPersistence::NotBootPersistent,
            },
        ];

        let first = evaluate_persistence_drift(&declared, &observed);
        let second = evaluate_persistence_drift(&declared, &observed);

        assert_eq!(first, second, "the predicate must be pure");
        assert_eq!(
            first[0].node_id, "alpha",
            "output must be sorted by node id"
        );
    }
}
