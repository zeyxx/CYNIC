//! Builder RBAC audit contract.
//!
//! The Builder node is deny-by-default when its scheduler principal cannot
//! modify promoted binaries, identities, or system configuration. This module
//! contains only pure observation reconciliation; host adapters collect facts,
//! and a bounded OneShotCapability applies any subsequent change.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_scheduler_write_access_to_promoted_binaries_identity_and_config() {
        let observation = BuilderRbacObservation {
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
