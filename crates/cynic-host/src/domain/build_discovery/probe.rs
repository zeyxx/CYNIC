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

/// The path a workspace-member PackageId points at.
///
/// A member id is a PackageId, not a name. This cargo emits
/// `path+file:///abs/path/crates/cynic-cli#0.8.0` — the fragment is the
/// *version*, the name is the last path segment, and there is no `name@`
/// component. Older cargo emitted `cynic-host 0.1.0 (path+file:///…)`. A bare
/// name is passed through.
fn member_path(id: &str) -> &str {
    let url = id.split('#').next().unwrap_or(id);
    let url = match (url.find('('), url.rfind(')')) {
        (Some(open), Some(close)) if close > open => &url[open + 1..close],
        _ => url,
    };
    let path = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    path.trim_end_matches('/')
}

/// Package directories referenced by workspace-member ids.
fn member_dirs(ids: &[String]) -> std::collections::HashSet<String> {
    ids.iter()
        .map(|id| member_path(id).to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

/// Names, derived from the last path segment. Best-effort fallback only: the
/// manifest-path join is authoritative. Deriving from the id fragment instead
/// would put versions like `0.8.0` into the name set, and stripping anything
/// after a dot would mangle legitimate names such as `crates.io-index`.
fn workspace_member_names(ids: &[String]) -> std::collections::HashSet<String> {
    ids.iter()
        .filter_map(|id| {
            let path = member_path(id);
            let name = if path.contains('/') {
                path.rsplit('/').next()?
            } else {
                path
            };
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn is_workspace_member(pkg: &MetadataPackage, dirs: &std::collections::HashSet<String>, names: &std::collections::HashSet<String>) -> bool {
    if dirs.is_empty() && names.is_empty() {
        return true;
    }
    if let Some(manifest) = &pkg.manifest_path {
        if let Some(parent) = std::path::Path::new(manifest).parent() {
            if dirs.contains(parent.to_string_lossy().as_ref()) {
                return true;
            }
        }
    }
    names.contains(&pkg.name)
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

    let ws_dirs = member_dirs(&meta.workspace_members);
    let ws_names = workspace_member_names(&meta.workspace_members);

    let mut crates = Vec::new();
    let mut packages_seen = 0usize;
    for pkg in &meta.packages {
        // With no member list there is nothing to filter against, so report
        // every package and let the caller see an honest superset.
        if !is_workspace_member(pkg, &ws_dirs, &ws_names) {
            continue;
        }
        packages_seen += 1;
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

    if crates.is_empty()
        && meta.packages.is_empty()
        && !residual_unknowns.contains(&"no_workspace_crates".to_string())
    {
        residual_unknowns.push("no_workspace_crates".to_string());
    }

    if crates.is_empty() && !meta.packages.is_empty() && packages_seen == 0 {
        // Naming the cause matters: the previous symptom-only check reported
        // `no_workspace_crates` here, which sent the reader looking for an
        // empty workspace instead of at the member-id matching below.
        residual_unknowns.push("workspace_member_ids_unmatched".to_string());
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

    /// The fixture above uses the legacy PackageId shape, which is why the test
    /// passed while real `cargo metadata` output produced zero crates: cargo
    /// stopped emitting that shape and the matching was never exercised against
    /// the one it now emits.
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
        assert_eq!(
            r.crates.len(),
            2,
            "both members must be found in the id shape cargo emits today"
        );
        assert!(r.crates.iter().any(|c| c.name == "cynic-cli"));
        assert!(r.crates.iter().any(|c| c.name == "cynic-host"));
        assert!(
            !r.crates.iter().any(|c| c.name == "serde"),
            "a non-member package must stay excluded"
        );
        assert!(r.residual_unknowns.is_empty(), "{:?}", r.residual_unknowns);
    }

    #[test]
    fn member_dirs_and_names_cover_every_known_id_shape() {
        let ids = vec![
            "path+file:///w/crates/cynic-cli#0.8.0".to_string(),
            "registry+https://github.com/rust-lang/crates.io-index#serde@1.0.197".to_string(),
            "cynic-host 0.1.0 (path+file:///w/crates/cynic-host)".to_string(),
            "cynic-kernel".to_string(),
        ];
        let dirs = member_dirs(&ids);
        assert!(dirs.contains("/w/crates/cynic-cli"), "current path id yields its dir");
        assert!(
            dirs.contains("/w/crates/cynic-host"),
            "legacy parenthesised id yields its dir"
        );
        let names = workspace_member_names(&ids);
        assert!(names.contains("cynic-cli"), "current path id yields its name");
        assert!(names.contains("cynic-host"), "legacy id yields its name");
        assert!(names.contains("cynic-kernel"), "bare id yields its name");
        assert!(
            names.contains("crates.io-index"),
            "a dotted segment is not truncated"
        );
        assert!(
            !names.contains("0.8.0") && !names.contains("1.0.197"),
            "a version is never offered as a name"
        );
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
        assert!(
            r.residual_unknowns.contains(&"workspace_member_ids_unmatched".to_string()),
            "the cause must be named: {:?}",
            r.residual_unknowns
        );
        assert!(
            !r.residual_unknowns.contains(&"no_workspace_crates".to_string()),
            "packages were present, so the workspace was not empty: {:?}",
            r.residual_unknowns
        );
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
