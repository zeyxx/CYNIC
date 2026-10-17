use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn audit_engine_has_no_orchestrator_dependencies() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-audit-engine violates downward DAG: depends on {}",
            forbidden
        );
    }
}

#[test]
fn audit_engine_has_no_uncontrolled_async_runtimes() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    assert!(
        !content.contains("tokio"),
        "cynic-audit-engine must not depend on tokio async runtime directly"
    );
}

#[test]
fn audit_engine_has_no_process_spawn_in_domain_core() {
    let src_dir = crate_root().join("src");
    let forbidden_patterns = ["std::process::Command", "reqwest::", "tokio::spawn"];

    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let content = fs::read_to_string(entry.path()).expect("read source file");
        for pattern in &forbidden_patterns {
            assert!(
                !content.contains(pattern),
                "cynic-audit-engine domain core {} contains forbidden I/O: {}",
                entry.path().display(),
                pattern
            );
        }
    }
}
