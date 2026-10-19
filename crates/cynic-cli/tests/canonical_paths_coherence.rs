use cynic_types::CynicPaths;
use std::path::Path;

#[test]
fn cynic_paths_defaults_match_directory_tree_contract() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let tree_path = repo_root.join("infra/directory-tree.json");

    let tree_content =
        std::fs::read_to_string(&tree_path).expect("infra/directory-tree.json must exist");
    let json: serde_json::Value = serde_json::from_str(&tree_content).expect("Valid JSON");

    let root_str = json["layers"]["installed_system"]["tree"]["root"]
        .as_str()
        .expect("installed_system.root must be declared");

    assert_eq!(
        root_str,
        CynicPaths::DEFAULT_SYSTEM_ROOT,
        "CynicPaths::DEFAULT_SYSTEM_ROOT must equal infra/directory-tree.json installed_system.root"
    );
}

#[test]
fn cynic_paths_environment_overrides_work_deterministically() {
    // 1. Default fallback
    assert_eq!(CynicPaths::DEFAULT_SYSTEM_ROOT, "/etc/cynic");
    assert_eq!(CynicPaths::DEFAULT_REGISTRY_FILENAME, "registry.json");
    assert_eq!(CynicPaths::DEFAULT_INFERENCE_FILENAME, "inference.toml");
    assert_eq!(CynicPaths::DEFAULT_SECRETS_DIRNAME, "secrets");

    // 2. Secret derivation
    assert_eq!(
        CynicPaths::secret_path("forgejo-token"),
        CynicPaths::secrets_dir().join("forgejo-token")
    );

    // 3. System root check
    assert!(CynicPaths::is_under_system_root(Path::new(
        "/etc/cynic/registry.json"
    )));
    assert!(!CynicPaths::is_under_system_root(Path::new(
        "/var/log/syslog"
    )));
}

#[test]
fn anti_regression_no_hardcoded_etc_in_cli_knowledge_surface() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let knowledge_surface_path = manifest_dir.join("src/knowledge_surface.rs");
    let content = std::fs::read_to_string(&knowledge_surface_path)
        .expect("src/knowledge_surface.rs must exist");

    assert!(
        !content.contains("\"/etc/cynic"),
        "crates/cynic-cli/src/knowledge_surface.rs must not hardcode '/etc/cynic', use CynicPaths SSOT"
    );
}
