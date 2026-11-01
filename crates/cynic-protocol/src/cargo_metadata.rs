//! The one argument vector used to ask Cargo about the workspace.
//!
//! Six adapters across `cynic-host` each spelled this command, in three
//! orthographies, one of which added `--frozen` and five of which did not. Discovery
//! could therefore disagree with itself depending on which adapter answered, which is
//! the same failure as `CORE_CRATES` existing in the Makefile: a fact that belongs to
//! the port, copied into every place that needed it.
//!
//! `--frozen` is the choice, and it is measured, not preferred: it is `--locked` plus
//! `--offline`, it returns the 48 real workspace members on this tree, and it matches
//! the cache-integrity check `check.layers` already performs first. It also removes the
//! network from the answer, so two runs of discovery cannot disagree because the
//! registry moved underneath one of them.
//!
//! Pure: an argument vector, no process, no filesystem. Lives in the L0 crate so that
//! `cynic-host` and any future packaging adapter read the same one.

/// Arguments for `cargo metadata`, excluding the program name.
pub const CARGO_METADATA_ARGS: &[&str] = &["metadata", "--format-version", "1", "--no-deps", "--frozen"];

/// The full argv, including the program, for adapters that want one slice.
///
/// Static by design: the arguments are a `const`, so handing the slice to an
/// adapter costs no allocation and borrows nothing.
///
/// The signature was `-> Vec<String>` until 2026-09-29. That contradicted the
/// line above it, allocated on every call for data that never outlived the call
/// (all four call sites immediately re-borrowed it into a `Vec<&str>`), and
/// broke four borrow-typed adapters. Owned argv, where an adapter genuinely
/// needs it, stays an adapter concern.
pub const CARGO_METADATA_ARGV: &[&str] = &[
    "cargo",
    "metadata",
    "--format-version",
    "1",
    "--no-deps",
    "--frozen",
];

/// The full argv, including the program, as one borrowable slice.
pub fn cargo_metadata_argv() -> &'static [&'static str] {
    CARGO_METADATA_ARGV
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vector_is_frozen_and_addresses_format_one() {
        assert!(
            CARGO_METADATA_ARGS.contains(&"--frozen"),
            "discovery must be offline and lock-checked, or two runs can disagree"
        );
        assert!(
            CARGO_METADATA_ARGS.windows(2).any(|w| w == ["--format-version", "1"]),
            "the split form is the one this tree is tested against; \
             the equals form is a second spelling of the same fact"
        );
        assert!(CARGO_METADATA_ARGS.contains(&"--no-deps"));
        assert_eq!(CARGO_METADATA_ARGS.first(), Some(&"metadata"));
    }

    #[test]
    fn argv_starts_with_the_program() {
        let argv = cargo_metadata_argv();
        assert_eq!(argv[0], "cargo");
        assert_eq!(argv.len(), CARGO_METADATA_ARGS.len() + 1);
    }

    /// The two constants are the same fact written twice; a drift between them
    /// would make the three `capture_command("cargo", ARGS)` adapters and the
    /// four `cargo_metadata_argv()` adapters disagree about the same command,
    /// which is exactly the failure this file was written to prevent.
    #[test]
    fn the_two_argv_forms_cannot_drift() {
        assert_eq!(&CARGO_METADATA_ARGV[1..], CARGO_METADATA_ARGS);
    }
}

use crate::build_discovery::{
    BUILD_DISCOVERY_REPORT_SCHEMA, BuildDiscoveryReport, DiscoveredCrate, HostTriple, Toolchain,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

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
pub fn member_dirs(ids: &[String]) -> HashSet<String> {
    ids.iter()
        .map(|id| member_path(id).to_string())
        .filter(|path| !path.is_empty())
        .collect()
}

/// Names, derived from the last path segment.
pub fn workspace_member_names(ids: &[String]) -> HashSet<String> {
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

fn is_workspace_member(
    pkg: &MetadataPackage,
    dirs: &HashSet<String>,
    names: &HashSet<String>,
) -> bool {
    if dirs.is_empty() && names.is_empty() {
        return true;
    }
    if let Some(manifest) = &pkg.manifest_path {
        if let Some(parent) = Path::new(manifest).parent() {
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
                    _ => {}
                }
            }
        }
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

    let targets = vec![host_triple.clone()];

    if crates.is_empty()
        && meta.packages.is_empty()
        && !residual_unknowns.contains(&"no_workspace_crates".to_string())
    {
        residual_unknowns.push("no_workspace_crates".to_string());
    }

    if crates.is_empty() && !meta.packages.is_empty() && packages_seen == 0 {
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

#[cfg(test)]
mod cargo_metadata_parser_tests {
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
        assert!(dirs.contains("/w/crates/cynic-cli"));
        assert!(dirs.contains("/w/crates/cynic-host"));
        let names = workspace_member_names(&ids);
        assert!(names.contains("cynic-cli"));
        assert!(names.contains("cynic-host"));
        assert!(names.contains("cynic-kernel"));
        assert!(names.contains("crates.io-index"));
        assert!(!names.contains("0.8.0") && !names.contains("1.0.197"));
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
        assert!(!r.residual_unknowns.contains(&"no_workspace_crates".to_string()));
    }

    #[test]
    fn parse_invalid_json_err() {
        assert!(parse_cargo_metadata("not json", 0, "x".into(), "y".into()).is_err());
    }
}
