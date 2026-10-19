//! Drift gate: external pins are the single SHA source, the project
//! registry references them by slug, and state roots stay canonical.
//!
//! Fails closed on: missing pin, SHA duplicated outside pins, state root
//! outside `/var/lib/cynic/projects/`, or `projects/` missing from the
//! directory tree.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn read_json(relative: &str) -> serde_json::Value {
    let raw = fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"));
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("invalid JSON {relative}: {err}"))
}

fn read_text(relative: &str) -> String {
    fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"))
}

#[test]
fn every_registry_project_resolves_to_a_pin() {
    let pins = read_json("infra/external-pins.json");
    let registry = read_json("infra/external-project-registry.json");

    let pin_slugs: BTreeSet<String> = pins["pins"]
        .as_array()
        .expect("pins array")
        .iter()
        .map(|p| {
            p["slug"].as_str().expect("pin slug").to_string()
        })
        .collect();

    let projects = registry["projects"]
        .as_array()
        .expect("projects array");
    assert!(!projects.is_empty(), "registry must declare projects");

    for project in projects {
        let slug = project["slug"].as_str().expect("project slug");
        let pin = &project["pin"];
        assert_eq!(
            pin["file"].as_str().expect("pin file"),
            "infra/external-pins.json",
            "project {slug} must reference the single pins file"
        );
        let pin_slug = pin["slug"].as_str().expect("pin slug");
        assert!(
            pin_slugs.contains(pin_slug),
            "project {slug} references unknown pin {pin_slug}"
        );
    }
}

#[test]
fn no_sha_lives_outside_the_pins_file() {
    let registry_text = read_text("infra/external-project-registry.json");
    assert!(
        !registry_text.contains("pin_sha"),
        "registry must reference pins by slug; SHAs live only in external-pins.json"
    );
}

#[test]
fn project_state_roots_stay_canonical() {
    let registry = read_json("infra/external-project-registry.json");
    let tree = read_text("infra/directory-tree.json");
    assert!(
        tree.contains("/var/lib/cynic/projects/"),
        "directory-tree.json must declare the projects/ pattern"
    );

    let projects = registry["projects"].as_array().expect("projects array");
    for project in projects {
        let slug = project["slug"].as_str().expect("project slug");
        let roots = &project["roots"];
        if let Some(state) = roots.get("state").and_then(|s| s.as_str()) {
            assert!(
                state.starts_with("/var/lib/cynic/projects/"),
                "project {slug} state root must live under /var/lib/cynic/projects/"
            );
        }
        if let Some(referential) = roots.get("referential").and_then(|s| s.as_str()) {
            assert!(
                referential.starts_with("/var/lib/cynic/source/repositories/external/"),
                "project {slug} referential must live under source/repositories/external/"
            );
        }
    }
}
