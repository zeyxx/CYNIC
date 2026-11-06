//! Reader for `infra/gc-surfaces.json` — the dynamic registry of
//! garbage-collection surfaces agents actually use.
//!
//! POURQUOI: `space_gc/observe.rs` hardcoded repo `target` patterns while the
//! 2026-10-19 operator audit measured 63G in `/tmp/cynic-*`, 25G in hermes
//! scratch dirs, 20G in tool caches, and 6G in logs — none covered.
//! COMMENT: Registry-first with compiled fallback (tmp_scratch + log_bundle
//! only). Pure file read + JSON parse; no FS walk, no deletion.
//! CE QU'IL NE FAIT PAS: never walks disks, never deletes, never grants
//! claim authority. Operator-owned surfaces (`claim_gate: true`) still need
//! explicit consent at execute time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const FALLBACK_SURFACES_JSON: &str = r#"{
    "surfaces": [
        {"id": "tmp_scratch", "kind": "tmp_scratch", "roots": ["/tmp"],
         "patterns": ["cynic-*", "verify-*", "sub-*"],
         "max_age_secs": 172800, "owner": "host",
         "claim_gate": false, "preserve": []},
        {"id": "log_bundle", "kind": "log_bundle", "roots": ["/var/log"],
         "patterns": ["kern.log.*", "ufw.log.*"],
         "max_age_secs": 1209600, "owner": "host",
         "claim_gate": false, "preserve": ["active"]}
    ]
}"#;

/// One GC surface from the registry.
#[derive(Debug, Clone, Deserialize)]
pub struct GcSurface {
    /// Stable surface id (`tmp_scratch`, `hermes_scratch`, ...).
    pub id: String,
    /// Maps to `cynic_protocol::retention::RetentionKind` by snake name.
    pub kind: String,
    /// Roots to scan. `{HOME}` expands to `$HOME` at observe time.
    #[serde(default)]
    pub roots: Vec<String>,
    /// Basename globs matched inside each root.
    #[serde(default)]
    pub patterns: Vec<String>,
    /// TTL seconds before a match is reclaimable.
    #[serde(default = "default_max_age")]
    pub max_age_secs: u64,
    /// Owning dimension (`host`, `node`, `operator`).
    #[serde(default = "default_owner")]
    pub owner: String,
    /// True means execute needs explicit human consent.
    #[serde(default)]
    pub claim_gate: bool,
    /// Basenames never collected.
    #[serde(default)]
    pub preserve: Vec<String>,
}

fn default_max_age() -> u64 {
    172800
}

fn default_owner() -> String {
    "host".to_string()
}

#[derive(Debug, Clone, Default, Deserialize)]
struct GcSurfacesRegistry {
    #[serde(default)]
    surfaces: Vec<GcSurface>,
    #[serde(default)]
    fallback: BTreeMap<String, String>,
}

/// Find the repository root by walking up from `start` for
/// `infra/gc-surfaces.json`. Returns `None` outside a worktree.
pub fn discover_registry_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if dir.join("infra").join("gc-surfaces.json").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// GC surfaces from the registry, or the compiled fallback when absent.
/// Never returns an empty vec: fallback covers tmp_scratch + log_bundle.
pub fn gc_surfaces(repo_root: Option<&Path>) -> Vec<GcSurface> {
    let from_registry = repo_root
        .and_then(|root| std::fs::read_to_string(root.join("infra").join("gc-surfaces.json")).ok())
        .and_then(|raw| serde_json::from_str::<GcSurfacesRegistry>(&raw).ok())
        .map(|registry| registry.surfaces)
        .filter(|surfaces| !surfaces.is_empty());
    match from_registry {
        Some(surfaces) => surfaces,
        _ => serde_json::from_str::<GcSurfacesRegistry>(FALLBACK_SURFACES_JSON)
            .map(|registry| registry.surfaces)
            .unwrap_or_default(),
    }
}

/// GC surfaces discovering the worktree from the current directory.
/// Outside a worktree this is the compiled fallback.
pub fn gc_surfaces_for_cwd() -> Vec<GcSurface> {
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| discover_registry_root(&cwd));
    gc_surfaces(root.as_deref())
}

/// Expand `{HOME}` in a registry root. Pure string rewrite, no FS touch.
pub fn expand_root(root: &str, home: &str) -> String {
    root.replace("{HOME}", home)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falls_back_outside_a_worktree() {
        let surfaces = gc_surfaces(None);
        assert!(surfaces.iter().any(|s| s.id == "tmp_scratch"));
        assert!(surfaces.iter().any(|s| s.id == "log_bundle"));
    }

    #[test]
    fn reads_surfaces_from_the_registry() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let infra = tmp.path().join("infra");
        std::fs::create_dir(&infra).expect("infra dir");
        std::fs::write(
            infra.join("gc-surfaces.json"),
            r#"{"surfaces": [{"id": "custom", "kind": "tmp_scratch", "roots": ["/tmp"], "patterns": ["custom-*"]}]}"#,
        )
        .expect("registry");
        let surfaces = gc_surfaces(Some(tmp.path()));
        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].id, "custom");
    }

    #[test]
    fn expands_home_token() {
        assert_eq!(
            expand_root("{HOME}/.cache", "/home/user"),
            "/home/user/.cache"
        );
        assert_eq!(expand_root("/tmp", "/home/user"), "/tmp");
    }
}
