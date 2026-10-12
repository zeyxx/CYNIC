use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn surreal_has_no_orchestrator_dependencies() {
    let cargo_toml_path = crate_root().join("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml_path).expect("read Cargo.toml");

    let forbidden_orchestrators = ["cynic-node", "cynic-cli", "cynic-kernel"];
    for forbidden in forbidden_orchestrators {
        assert!(
            !content.contains(&format!("\"{}\"", forbidden)),
            "cynic-surreal violates downward DAG: depends on {}",
            forbidden
        );
    }
}

#[test]
fn surreal_domain_core_has_zero_io() {
    let domain_dir = crate_root().join("src").join("domain");
    let forbidden_io = [
        "reqwest::",
        "std::process::Command",
        "tokio::spawn",
        "std::net",
    ];

    for entry in walkdir::WalkDir::new(&domain_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let content = fs::read_to_string(entry.path()).expect("read domain file");
        for pattern in forbidden_io {
            assert!(
                !content.contains(pattern),
                "FCIS violation: domain core file {:?} contains I/O pattern '{}'",
                entry.path(),
                pattern
            );
        }
    }
}
