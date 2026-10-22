//! Coherence between `infra/external-project-registry.json` and the tier model
//! declared in `infra/workspace-paths.json`.
//!
//! WHY: `serving_tier` is the SSOT of an external project's checkout root.
//! `roots.referential` must equal f(serving_tier) against production_root /
//! development_root. A mismatch means the registry and the disk disagree on
//! where the project lives — the exact drift this file exists to prevent.
//!
//! This is an architecture test (like `workspace_paths_coherence.rs`): it lives
//! in `cynic-cli` because it needs I/O to read the registries.

use std::env;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
}

fn read_json(relative: &str) -> serde_json::Value {
    let path = repo_root().join(relative);
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn expand_home(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("${HOME}") {
        format!("{}{}", env::var("HOME").unwrap_or_default(), rest)
    } else {
        s.to_string()
    }
}

#[test]
fn every_external_project_declares_a_valid_serving_tier() {
    let registry = read_json("infra/external-project-registry.json");
    let projects = registry["projects"].as_array().expect("projects must be an array");
    assert!(!projects.is_empty(), "registry must declare at least one project");

    for p in projects {
        let slug = p["slug"].as_str().expect("slug must be a string");
        let tier = p["serving_tier"]
            .as_str()
            .unwrap_or_else(|| panic!("project {slug} is missing serving_tier"));
        assert!(
            tier == "served" || tier == "dev",
            "project {slug} has invalid serving_tier: {tier}"
        );
    }
}

#[test]
fn external_project_referential_root_matches_serving_tier() {
    let paths = read_json("infra/workspace-paths.json");
    let ext = &paths["external_projects"];
    let production_root = expand_home(ext["production_root"].as_str().expect("production_root"));
    let development_root = expand_home(ext["development_root"].as_str().expect("development_root"));

    let registry = read_json("infra/external-project-registry.json");
    let projects = registry["projects"].as_array().expect("projects must be an array");

    for p in projects {
        let slug = p["slug"].as_str().expect("slug must be a string");
        let tier = p["serving_tier"].as_str().expect("serving_tier must be set");
        let referential = expand_home(
            p["roots"]["referential"]
                .as_str()
                .unwrap_or_else(|| panic!("project {slug} is missing roots.referential")),
        );

        let expected = match tier {
            "served" => format!("{}/{}", production_root, slug),
            "dev" => format!("{}/{}", development_root, slug),
            _ => panic!("invalid tier {tier} for {slug}"),
        };

        assert_eq!(
            referential, expected,
            "project {slug}: roots.referential ({referential}) != f(serving_tier={tier}) ({expected})"
        );
    }
}

#[test]
fn external_project_registry_and_pins_agree_on_slugs() {
    let registry = read_json("infra/external-project-registry.json");
    let pins = read_json("infra/external-pins.json");

    let reg_slugs: std::collections::BTreeSet<&str> = registry["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["slug"].as_str())
        .collect();

    let pin_slugs: std::collections::BTreeSet<&str> = pins["pins"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|p| p["slug"].as_str())
        .collect();

    // Every registered project must have a pin.
    for slug in &reg_slugs {
        assert!(
            pin_slugs.contains(slug),
            "project {slug} is in the registry but has no pin in infra/external-pins.json"
        );
    }
}