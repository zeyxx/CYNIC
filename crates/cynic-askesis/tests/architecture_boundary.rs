use std::fs;
use std::path::PathBuf;

fn askesis_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn askesis_domain_has_no_orchestrator_dependencies() {
    let cargo_toml_path = askesis_crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-host", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-askesis violates downward DAG: depends on orchestrator {}",
            forbidden
        );
    }
}

#[test]
fn askesis_lib_is_bounded_facade() {
    let lib_rs = askesis_crate_root().join("src/lib.rs");
    let content = fs::read_to_string(&lib_rs).expect("read src/lib.rs");
    let lines = content.lines().count();
    assert!(
        lines <= 220,
        "cynic-askesis entrypoint src/lib.rs is too large: {lines} lines (threshold: 220)"
    );
}
