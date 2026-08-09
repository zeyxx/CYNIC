fn api_topology_contract_paths(project_root: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = PathBuf::from(project_root);
    (
        root.join("docs/api/openapi.json"),
        root.join("axioms/api-topology-v1.json"),
        root.join("axioms/api-layer-alignment-v1.json"),
        root.join("crates/cynic-kernel/src/api/rest/discovery.rs"),
        root.join("crates/cynic-kernel/src/api/rest/middleware.rs"),
    )
}
