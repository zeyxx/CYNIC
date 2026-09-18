use std::fs;
use std::path::PathBuf;

fn lxc_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn lxc_domain_has_no_orchestrator_dependencies() {
    let cargo_toml_path = lxc_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-host", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-lxc violates downward DAG: depends on orchestrator {}",
            forbidden
        );
    }
}

#[test]
fn lxc_decision_has_zero_direct_process_calls() {
    let decision_rs = lxc_crate_root().join("src/decision.rs");
    let content = fs::read_to_string(&decision_rs).expect("read decision.rs");

    assert!(
        !content.contains("std::process::Command"),
        "cynic-lxc decision engine must be pure without direct Command execution"
    );
}
