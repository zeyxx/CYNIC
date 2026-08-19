pub mod apply;
pub mod observe;
pub mod reconcile;
pub mod types;

pub use apply::apply_host_hardening;
pub use observe::{observe_firewall_state, observe_sshd_state};
pub use reconcile::plan_host_hardening;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::hardening::HardeningPolicyLevel;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_host_hardening_plan_and_reconciliation() {
        let mut temp_sshd = NamedTempFile::new().expect("temp sshd");
        writeln!(
            temp_sshd,
            "PasswordAuthentication yes\nPermitRootLogin yes\nPort 22"
        )
        .expect("write");

        let plan = plan_host_hardening(
            HardeningPolicyLevel::StrictSovereign,
            Some(temp_sshd.path().to_str().unwrap()),
        );

        assert!(!plan.is_converged);
        assert!(plan.pending_actions.len() >= 2);
        assert!(plan.pending_actions.iter().any(|a| a.id == "SSH-DISABLE-PASSWORD-AUTH"));
        assert!(plan.pending_actions.iter().any(|a| a.id == "SSH-PROHIBIT-ROOT-PASSWORD"));

        // Dry-run execution
        let dry_proof = apply_host_hardening(&plan, true).expect("dry run");
        assert_eq!(dry_proof.status, "dry_run_planned");
        assert_eq!(dry_proof.actions_applied, 0);

        // Real execution mock
        let apply_proof = apply_host_hardening(&plan, false).expect("apply run");
        assert_eq!(apply_proof.status, "applied_successfully");
        assert_eq!(apply_proof.actions_applied, plan.pending_actions.len());
    }
}
