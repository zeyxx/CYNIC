//! Builder RBAC audit contract.
//!
//! The Builder node is deny-by-default when its scheduler principal cannot
//! modify promoted binaries, identities, or system configuration. This module
//! contains only pure observation reconciliation; host adapters collect facts,
//! and a bounded OneShotCapability applies any subsequent change.

use serde::{Deserialize, Serialize};

pub const BUILDER_RBAC_OBSERVATION_SCHEMA: &str = "cynic.builder_rbac_observation.v1";
pub const BUILDER_RBAC_DECISION_SCHEMA: &str = "cynic.builder_rbac_decision.v1";

/// Minimal observed authority surface of the Builder scheduler principal.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuilderRbacObservation {
    pub schema: String,
    pub node_principal: String,
    /// Paths writable by the long-running scheduler service.
    pub node_write_paths: Vec<String>,
    /// Unit hardening facts; `None` is explicit unknown.
    pub protect_system: Option<String>,
    pub no_new_privileges: Option<bool>,
    pub capability_bounding_set: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuilderRbacStatus {
    Ready,
    Blocked,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuilderRbacDecision {
    pub schema: String,
    pub status: BuilderRbacStatus,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

/// Pure reconciliation of the Builder node’s write authority.
///
/// The scheduler may later receive explicitly declared state/workspace roots,
/// but it must never write promoted binaries, identities, or system config.
pub fn reconcile_builder_rbac(observation: &BuilderRbacObservation) -> BuilderRbacDecision {
    const FORBIDDEN_NODE_WRITE_ROOTS: [&str; 3] =
        ["/var/lib/cynic/bin", "/var/lib/cynic/.ssh", "/etc/cynic"];

    let mut blockers = Vec::new();
    let mut residual_unknowns = Vec::new();
    if observation.schema != BUILDER_RBAC_OBSERVATION_SCHEMA {
        blockers.push("builder_rbac_observation_schema_mismatch".to_string());
    }
    if observation.node_principal.trim().is_empty() {
        blockers.push("builder_node_principal_unknown".to_string());
    }
    for root in FORBIDDEN_NODE_WRITE_ROOTS {
        if observation
            .node_write_paths
            .iter()
            .any(|path| path == root || path.starts_with(&format!("{root}/")))
        {
            blockers.push(format!("node_write_forbidden:{root}"));
        }
    }
    match observation.protect_system.as_deref() {
        Some("strict") => {}
        Some(_) => blockers.push("builder_protect_system_not_strict".to_string()),
        None => residual_unknowns.push("builder_protect_system_unknown".to_string()),
    }
    match observation.no_new_privileges {
        Some(true) => {}
        Some(false) => blockers.push("builder_no_new_privileges_disabled".to_string()),
        None => residual_unknowns.push("builder_no_new_privileges_unknown".to_string()),
    }
    blockers.sort();
    blockers.dedup();
    residual_unknowns.sort();
    residual_unknowns.dedup();
    let status = if !blockers.is_empty() {
        BuilderRbacStatus::Blocked
    } else if !residual_unknowns.is_empty() {
        BuilderRbacStatus::Unknown
    } else {
        BuilderRbacStatus::Ready
    };
    BuilderRbacDecision {
        schema: BUILDER_RBAC_DECISION_SCHEMA.to_string(),
        status,
        blockers,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_scheduler_write_access_to_promoted_binaries_identity_and_config() {
        let observation = BuilderRbacObservation {
            schema: BUILDER_RBAC_OBSERVATION_SCHEMA.to_string(),
            node_principal: "cynic".to_string(),
            node_write_paths: vec![
                "/var/lib/cynic/bin".to_string(),
                "/var/lib/cynic/.ssh".to_string(),
                "/etc/cynic".to_string(),
            ],
            ..Default::default()
        };

        let decision = reconcile_builder_rbac(&observation);
        assert_eq!(decision.status, BuilderRbacStatus::Blocked);
        assert!(
            decision
                .blockers
                .contains(&"node_write_forbidden:/var/lib/cynic/bin".to_string())
        );
        assert!(
            decision
                .blockers
                .contains(&"node_write_forbidden:/var/lib/cynic/.ssh".to_string())
        );
        assert!(
            decision
                .blockers
                .contains(&"node_write_forbidden:/etc/cynic".to_string())
        );
    }
}
