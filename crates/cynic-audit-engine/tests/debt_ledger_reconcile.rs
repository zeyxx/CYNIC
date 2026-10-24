#![allow(clippy::expect_used)]

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return dir;
        }
        assert!(dir.pop(), "workspace root not found");
    }
}

#[test]
#[ignore]
fn reconcile_debt_ledger_from_substrate() {
    let root = workspace_root();
    let count = cynic_audit_engine::reconcile_debt_ledger(&root).expect("reconcile debt ledger");
    println!("Reconciled {} debt ledger identities.", count);

    // Verify that applying the ledger produces a clean verdict
    let report = cynic_audit_engine::run_workspace_audit(&root);
    assert_eq!(
        report.summary.status, "ok",
        "Report status after reconciliation must be ok, got: {} with {} critical violations: {:?}",
        report.summary.status, report.summary.critical_count, report.violations
    );
    assert_eq!(report.summary.critical_count, 0);
}
