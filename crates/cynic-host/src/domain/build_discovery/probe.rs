//! Pure parsers for build discovery (no I/O).

use cynic_build::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, DiscoveredCrate, HostTriple, Toolchain,
};
use cynic_build::build_discovery_recipe::{BUILD_DISCOVERY_RECIPE_SCHEMA, BuildDiscoveryRecipe};
use serde::Deserialize;

/// Minimal cargo metadata shape (only fields we need).
#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
    #[serde(default)]
    workspace_members: Vec<String>,
    #[serde(default)]
    #[allow(dead_code)]
    target_directory: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    #[serde(default)]
    edition: Option<String>,
    #[serde(default)]
    targets: Vec<MetadataTarget>,
    #[serde(default)]
    manifest_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    name: String,
    kind: Vec<String>,
}

/// Parse `cargo metadata --format-version=1 --no-deps` JSON into a report.
/// Pure, no FS/process touch.
pub fn parse_cargo_metadata(
    json: &str,
    now: u64,
    toolchain: String,
    host_triple: String,
) -> Result<BuildDiscoveryReport, String> {
    let meta: CargoMetadata =
        serde_json::from_str(json).map_err(|e| format!("cargo_metadata_parse: {e}"))?;

    let mut residual_unknowns = Vec::new();
    if meta.packages.is_empty() {
        residual_unknowns.push("cargo_metadata_empty_packages".to_string());
    }
    if meta.workspace_members.is_empty() {
        residual_unknowns.push("workspace_members_unknown".to_string());
    }

    let ws_set: std::collections::HashSet<String> =
        meta.workspace_members.iter().cloned().collect();

    let mut crates = Vec::new();
    for pkg in &meta.packages {
        // Only include workspace members if list non-empty; else include all.
        let is_member = if ws_set.is_empty() {
            true
        } else {
            // workspace_members are PackageId like "cynic-host 0.1.0 (path+file://...)"
            // match by prefix "name "
            ws_set
                .iter()
                .any(|m| m.starts_with(&format!("{} ", pkg.name)))
                || ws_set.contains(&pkg.name)
        };
        if !is_member {
            continue;
        }
        let edition = pkg.edition.clone().unwrap_or_else(|| "unknown".to_string());
        let mut bins = Vec::new();
        let mut libs = Vec::new();
        let mut tests = Vec::new();
        let mut custom_build = false;
        for t in &pkg.targets {
            for k in &t.kind {
                match k.as_str() {
                    "bin" => bins.push(t.name.clone()),
                    "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro" => {
                        if !libs.contains(&t.name) {
                            libs.push(t.name.clone());
                        }
                    }
                    "test" => tests.push(t.name.clone()),
                    "custom-build" => custom_build = true,
                    // bench, example ignored
                    _ => {}
                }
            }
        }
        // dedup bins/tests (targets may repeat kinds)
        bins.sort();
        bins.dedup();
        libs.sort();
        libs.dedup();
        tests.sort();
        tests.dedup();

        crates.push(DiscoveredCrate {
            name: pkg.name.clone(),
            edition,
            bins,
            libs,
            tests,
            custom_build,
        });
    }
    crates.sort_by(|a, b| a.name.cmp(&b.name));

    // targets: single host triple as observed target
    let targets = vec![host_triple.clone()];

    if crates.is_empty() && !residual_unknowns.contains(&"no_workspace_crates".to_string()) {
        residual_unknowns.push("no_workspace_crates".to_string());
    }

    Ok(BuildDiscoveryReport {
        schema: BUILD_DISCOVERY_REPORT_SCHEMA.to_string(),
        toolchain: Toolchain(toolchain),
        host_triple: HostTriple(host_triple),
        targets,
        crates,
        observed_at_unix_s: now,
        residual_unknowns,
    })
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
        // Crate with no bin/lib/test but still buildable -> generic cargo build -p
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
            {"name":"cynic-host","edition":"2024","targets":[
                {"name":"cynic-host","kind":["lib"]},
                {"name":"mybin","kind":["bin"]}
            ]},
            {"name":"cynic-protocol","edition":"2024","targets":[
                {"name":"cynic-protocol","kind":["lib"]},
                {"name":"build","kind":["custom-build"]}
            ]},
            {"name":"foo","edition":"2021","targets":[
                {"name":"foo-test","kind":["test"]}
            ]}
        ],
        "workspace_members":["cynic-host 0.1.0 (path+file:///tmp)","cynic-protocol 0.1.0 (path+file:///tmp)"],
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
        assert!(
            recipes
                .iter()
                .any(|x| x.id == "build:cynic-host:lib:cynic-host")
        );
        assert!(
            recipes
                .iter()
                .any(|x| x.id == "build:cynic-protocol:custom-build")
        );
        // foo not in workspace -> no recipe
        assert!(!recipes.iter().any(|x| x.crate_name == "foo"));
    }

    #[test]
    fn parse_invalid_json_err() {
        assert!(parse_cargo_metadata("not json", 0, "x".into(), "y".into()).is_err());
    }
}
