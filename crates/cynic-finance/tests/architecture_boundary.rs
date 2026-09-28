use std::fs;
use std::path::PathBuf;

fn finance_crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn finance_domain_is_pure_zero_io() {
    let cargo_toml =
        fs::read_to_string(finance_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    let forbidden_crates = ["cynic-host", "cynic-kernel", "cynic-node", "cynic-cli"];
    for forbidden in &forbidden_crates {
        assert!(
            !cargo_toml.contains(forbidden),
            "Architecture violation: cynic-finance must never depend on orchestrator or host crates ({forbidden})"
        );
    }
}

#[test]
fn finance_has_no_std_process_or_reqwest() {
    let src_dir = finance_crate_root().join("src");
    let entries = fs::read_dir(&src_dir).expect("read src dir");

    for entry in entries {
        let entry = entry.expect("read entry");
        let content = fs::read_to_string(entry.path()).expect("read source file");

        assert!(
            !content.contains("std::process::Command"),
            "Architecture violation: cynic-finance must not use std::process::Command in {}",
            entry.path().display()
        );
        assert!(
            !content.contains("reqwest"),
            "Architecture violation: cynic-finance must not use reqwest in {}",
            entry.path().display()
        );
    }
}

#[test]
fn finance_depends_on_protocol() {
    let cargo_toml =
        fs::read_to_string(finance_crate_root().join("Cargo.toml")).expect("read Cargo.toml");

    assert!(
        cargo_toml.contains("cynic-protocol"),
        "cynic-finance must depend on cynic-protocol for shared types"
    );
}
