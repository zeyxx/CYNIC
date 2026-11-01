//! Pure parsers for build discovery (no I/O).

use cynic_protocol::build_discovery::BuildDiscoveryReport;
use cynic_protocol::build_discovery_recipe::{BUILD_DISCOVERY_RECIPE_SCHEMA, BuildDiscoveryRecipe};
use cynic_protocol::cargo_metadata::parse_cargo_metadata as parse_cargo_metadata_impl;

/// Parse `cargo metadata --format-version=1 --no-deps` JSON into a report.
/// Pure, no FS/process touch. Delegated to `cynic_protocol::cargo_metadata`.
pub fn parse_cargo_metadata(
    json: &str,
    now: u64,
    toolchain: String,
    host_triple: String,
) -> Result<BuildDiscoveryReport, String> {
    parse_cargo_metadata_impl(json, now, toolchain, host_triple)
}

/// Pure recipe derivation from a report (no I/O).
pub fn derive_recipes(report: &BuildDiscoveryReport) -> Vec<BuildDiscoveryRecipe> {
    let mut out = Vec::new();
    for krate in &report.crates {
        for bin in &krate.bins {
            out.push(BuildDiscoveryRecipe {
                schema: BUILD_DISCOVERY_RECIPE_SCHEMA.to_string(),
                id: format!("build:{}:bin:{}", krate.name, bin),
                action: "cargo_build".to_string(),
                crate_name: krate.name.clone(),
                target_kind: "bin".to_string(),
                target_name: bin.clone(),
                argv: vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-p".to_string(),
                    krate.name.clone(),
                    "--bin".to_string(),
                    bin.clone(),
                ],
            });
        }
        for lib in &krate.libs {
            out.push(BuildDiscoveryRecipe {
                schema: BUILD_DISCOVERY_RECIPE_SCHEMA.to_string(),
                id: format!("build:{}:lib:{}", krate.name, lib),
                action: "cargo_build".to_string(),
                crate_name: krate.name.clone(),
                target_kind: "lib".to_string(),
                target_name: lib.clone(),
                argv: vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-p".to_string(),
                    krate.name.clone(),
                    "--lib".to_string(),
                ],
            });
        }
        for test in &krate.tests {
            out.push(BuildDiscoveryRecipe {
                schema: BUILD_DISCOVERY_RECIPE_SCHEMA.to_string(),
                id: format!("build:{}:test:{}", krate.name, test),
                action: "cargo_test".to_string(),
                crate_name: krate.name.clone(),
                target_kind: "test".to_string(),
                target_name: test.clone(),
                argv: vec![
                    "cargo".to_string(),
                    "test".to_string(),
                    "-p".to_string(),
                    krate.name.clone(),
                    "--test".to_string(),
                    test.clone(),
                    "--no-run".to_string(),
                ],
            });
        }
        if krate.custom_build {
            out.push(BuildDiscoveryRecipe {
                schema: BUILD_DISCOVERY_RECIPE_SCHEMA.to_string(),
                id: format!("build:{}:custom-build", krate.name),
                action: "cargo_build".to_string(),
                crate_name: krate.name.clone(),
                target_kind: "custom-build".to_string(),
                target_name: krate.name.clone(),
                argv: vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-p".to_string(),
                    krate.name.clone(),
                ],
            });
        }
        if krate.bins.is_empty()
            && krate.libs.is_empty()
            && krate.tests.is_empty()
            && !krate.custom_build
        {
            out.push(BuildDiscoveryRecipe {
                schema: BUILD_DISCOVERY_RECIPE_SCHEMA.to_string(),
                id: format!("build:{}:crate", krate.name),
                action: "cargo_build".to_string(),
                crate_name: krate.name.clone(),
                target_kind: "crate".to_string(),
                target_name: krate.name.clone(),
                argv: vec![
                    "cargo".to_string(),
                    "build".to_string(),
                    "-p".to_string(),
                    krate.name.clone(),
                ],
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "packages": [
            {"name":"cynic-host","edition":"2024","manifest_path":"/tmp/cynic-host/Cargo.toml","targets":[
                {"name":"cynic-host","kind":["lib"]},
                {"name":"mybin","kind":["bin"]}
            ]},
            {"name":"cynic-protocol","edition":"2024","manifest_path":"/tmp/cynic-protocol/Cargo.toml","targets":[
                {"name":"cynic-protocol","kind":["lib"]},
                {"name":"build","kind":["custom-build"]}
            ]},
            {"name":"foo","edition":"2021","manifest_path":"/tmp/foo/Cargo.toml","targets":[
                {"name":"foo-test","kind":["test"]}
            ]}
        ],
        "workspace_members":["cynic-host 0.1.0 (path+file:///tmp/cynic-host)","cynic-protocol 0.1.0 (path+file:///tmp/cynic-protocol)"],
        "target_directory":"/tmp/target",
        "version":1
    }"#;

    #[test]
    fn parse_filters_workspace_members() {
        let r = parse_cargo_metadata(
            SAMPLE,
            123,
            "stable".into(),
            "x86_64-unknown-linux-gnu".into(),
        )
        .unwrap();
        assert_eq!(r.crates.len(), 2);
        assert!(r.crates.iter().any(|c| c.name == "cynic-host"));
        assert_eq!(r.observed_at_unix_s, 123);
        assert_eq!(r.targets, vec!["x86_64-unknown-linux-gnu"]);
    }

    #[test]
    fn parse_handles_every_current_package_id_shape() {
        let json = r#"{
            "packages": [
                {"name":"cynic-cli","edition":"2024","manifest_path":"/w/crates/cynic-cli/Cargo.toml","targets":[{"name":"cynic","kind":["bin"]}]},
                {"name":"cynic-host","edition":"2024","manifest_path":"/w/crates/cynic-host/Cargo.toml","targets":[{"name":"cynic-host","kind":["lib"]}]},
                {"name":"serde","edition":"2021","manifest_path":"/h/serde/Cargo.toml","targets":[{"name":"serde","kind":["lib"]}]}
            ],
                "workspace_members":[
                "path+file:///w/crates/cynic-cli#0.8.0",
                "path+file:///w/crates/cynic-host#0.8.0"
            ],
            "version":1
        }"#;
        let r = parse_cargo_metadata(json, 1, "1.95.0".into(), "x86_64-unknown-linux-gnu".into())
            .unwrap();
        assert_eq!(r.crates.len(), 2);
        assert!(r.crates.iter().any(|c| c.name == "cynic-cli"));
        assert!(r.crates.iter().any(|c| c.name == "cynic-host"));
        assert!(!r.crates.iter().any(|c| c.name == "serde"));
    }

    #[test]
    fn unmatched_member_ids_are_named_rather_than_reported_as_no_crates() {
        let json = r#"{
            "packages": [
                {"name":"cynic-cli","edition":"2024","targets":[{"name":"cynic","kind":["bin"]}]}
            ],
            "workspace_members":["a-shape-nobody-emits"],
            "version":1
        }"#;
        let r = parse_cargo_metadata(json, 1, "1.95.0".into(), "x86_64-unknown-linux-gnu".into())
            .unwrap();
        assert!(r.crates.is_empty());
        assert!(r.residual_unknowns.contains(&"workspace_member_ids_unmatched".to_string()));
    }

    #[test]
    fn parse_invalid_json_err() {
        assert!(parse_cargo_metadata("not json", 0, "x".into(), "y".into()).is_err());
    }

    #[test]
    fn derive_recipes_bins_libs() {
        let r = parse_cargo_metadata(
            SAMPLE,
            0,
            "stable".into(),
            "x86_64-unknown-linux-gnu".into(),
        )
        .unwrap();
        let recipes = derive_recipes(&r);
        assert!(recipes.iter().any(|x| x.id == "build:cynic-host:bin:mybin"));
        assert!(recipes.iter().any(|x| x.id == "build:cynic-host:lib:cynic-host"));
        assert!(recipes.iter().any(|x| x.id == "build:cynic-protocol:custom-build"));
        assert!(!recipes.iter().any(|x| x.crate_name == "foo"));
    }
}
