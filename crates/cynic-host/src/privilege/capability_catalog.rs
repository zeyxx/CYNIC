//! Capability catalog adapter — load the declared catalog and the installed
//! set from disk, then admit through the pure `cynic-protocol` decision.
//!
//! The policy is DATA. This module reads it, never authors it. Admission is
//! the single pure function in `cynic_protocol::privilege`; this adapter only
//! supplies the two documents it needs and surfaces the result.
//!
//! One catalog, one `admit()`. A domain that builds its own argv is a second
//! source of truth, and the measured cost of that was three implementations
//! of the same `pct` argv diverging silently.

use std::path::{Path, PathBuf};

use cynic_protocol::privilege::{
    CapabilityCatalog, CapabilityDecision, InstalledCapabilities, admit_capability,
    validate_install,
};
use cynic_types::paths::CynicPaths;

/// Repository-relative default catalog.
pub const DEFAULT_CATALOG_PATH: &str = "infra/capabilities/catalog.json";

/// Node-local installed set, written by `cynic install`:
/// `capabilities.installed.json` under the canonical system config directory,
/// unless `CYNIC_CAPABILITY_INSTALLED` overrides it.

/// The loaded pair, validated together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityView {
    pub catalog: CapabilityCatalog,
    pub installed: InstalledCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityLoadError(pub String);

impl std::fmt::Display for CapabilityLoadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "capability catalog error: {}", self.0)
    }
}

impl std::error::Error for CapabilityLoadError {}

impl CapabilityView {
    /// Load the catalog and the installed set, validating both. A catalog
    /// that fails structural validation is not a permissive catalog: it is an
    /// error, and no capability is admitted from it.
    pub fn load(
        catalog_path: impl AsRef<Path>,
        installed_path: impl AsRef<Path>,
    ) -> Result<Self, CapabilityLoadError> {
        let catalog_path = catalog_path.as_ref();
        let installed_path = installed_path.as_ref();

        let catalog_raw = std::fs::read_to_string(catalog_path).map_err(|error| {
            CapabilityLoadError(format!("read {}: {error}", catalog_path.display()))
        })?;
        let catalog: CapabilityCatalog = serde_json::from_str(&catalog_raw).map_err(|error| {
            CapabilityLoadError(format!("parse {}: {error}", catalog_path.display()))
        })?;
        let structural = catalog.validate();
        if !structural.is_empty() {
            return Err(CapabilityLoadError(format!(
                "catalog {} is invalid: {}",
                catalog_path.display(),
                structural.join(", ")
            )));
        }

        let installed_raw = std::fs::read_to_string(installed_path).map_err(|error| {
            CapabilityLoadError(format!("read {}: {error}", installed_path.display()))
        })?;
        let installed: InstalledCapabilities =
            serde_json::from_str(&installed_raw).map_err(|error| {
                CapabilityLoadError(format!("parse {}: {error}", installed_path.display()))
            })?;

        let install_blockers = validate_install(&catalog, &installed);
        if !install_blockers.is_empty() {
            return Err(CapabilityLoadError(format!(
                "installed set {} is invalid: {}",
                installed_path.display(),
                install_blockers.join(", ")
            )));
        }

        Ok(Self { catalog, installed })
    }

    /// Admit a capability id with its parameters.
    pub fn admit(&self, id: &str, parameters: &[(String, String)]) -> CapabilityDecision {
        admit_capability(&self.catalog, &self.installed, id, parameters)
    }

    /// Ids the catalog requires on every node.
    pub fn mandatory_ids(&self) -> Vec<String> {
        self.catalog.mandatory_ids()
    }
}

/// Resolve the catalog path.
///
/// Precedence: explicit env override, the installed copy under the
/// runtime config root, then the repository copy found by walking up
/// from the working directory (developer checkouts).
///
/// The previous resolution tried the repository path RELATIVE to the
/// working directory. An installed binary started from `/` therefore
/// found no catalogue at all, and a process that cannot find its
/// catalogue is a process with no membrane. The worktree copy is now
/// located by walking up the ancestors, the way the registry is, and the
/// installed copy is named by the runtime authority (`config_root`)
/// rather than by a literal path.
///
/// The final fallback is the installed location, so the error message
/// names where the catalogue is expected to be.
pub fn catalog_path_from_env() -> PathBuf {
    if let Some(explicit) = std::env::var_os("CYNIC_CAPABILITY_CATALOG") {
        return PathBuf::from(explicit);
    }
    let installed = CynicPaths::system_config_dir().join(INSTALLED_CATALOG_RELATIVE_PATH);
    if installed.is_file() {
        return installed;
    }
    if let Some(found) = find_catalog_in_ancestors() {
        return found;
    }
    installed
}

/// The catalog path relative to the runtime config root.
pub const INSTALLED_CATALOG_RELATIVE_PATH: &str = "capabilities/catalog.json";

/// Find the repository catalogue by walking up from the working directory.
///
/// A developer checkout may be entered at any depth; the catalogue
/// lives at the root of the tree. The walk mirrors the one the registry
/// uses, so both authorities resolve the same way.
fn find_catalog_in_ancestors() -> Option<PathBuf> {
    let current = std::env::current_dir().ok()?;
    for ancestor in current.ancestors() {
        let candidate = ancestor.join(DEFAULT_CATALOG_PATH);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Resolve the installed-set path.
pub fn installed_path_from_env() -> PathBuf {
    std::env::var_os("CYNIC_CAPABILITY_INSTALLED")
        .map(PathBuf::from)
        .unwrap_or_else(|| CynicPaths::system_config_dir().join("capabilities.installed.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(path: &Path, contents: &str) {
        let mut file = std::fs::File::create(path).expect("create");
        file.write_all(contents.as_bytes()).expect("write");
    }

    fn catalog_json() -> &'static str {
        r#"{
          "schema": "cynic.capability_catalog.v1",
          "capabilities": [
            {
              "id": "node-control",
              "domain": "runtime",
              "argv_template": ["node", "control"],
              "params": [],
              "requires_root": false,
              "mandatory": true,
              "purpose": "supervised node runtime"
            },
            {
              "id": "pct-list",
              "domain": "hypervisor",
              "argv_template": ["/usr/sbin/pct", "list"],
              "params": [],
              "requires_root": true,
              "mandatory": false,
              "purpose": "CT inventory"
            }
          ]
        }"#
    }

    fn installed_json(ids: &str) -> String {
        format!(
            r#"{{"schema":"cynic.capability_catalog.installed.v1","capability_ids":[{ids}]}}"#
        )
    }

    #[test]
    fn loads_and_admits() {
        let dir = std::env::temp_dir().join(format!("cynic-cap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let catalog = dir.join("catalog.json");
        let installed = dir.join("installed.json");
        write(&catalog, catalog_json());
        write(&installed, &installed_json(r#""node-control","pct-list""#));

        let view = CapabilityView::load(&catalog, &installed).expect("load");
        assert!(view.mandatory_ids().contains(&"node-control".to_string()));
        match view.admit("pct-list", &[]) {
            CapabilityDecision::Admitted { argv, requires_root, .. } => {
                assert_eq!(argv, vec!["/usr/sbin/pct", "list"]);
                assert!(requires_root);
            }
            other => panic!("expected admitted, got {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_mandatory_is_a_load_error() {
        let dir = std::env::temp_dir().join(format!("cynic-cap-m-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let catalog = dir.join("catalog.json");
        let installed = dir.join("installed.json");
        write(&catalog, catalog_json());
        write(&installed, &installed_json(r#""pct-list""#));

        let error = CapabilityView::load(&catalog, &installed).expect_err("must fail");
        assert!(error.0.contains("mandatory_capability_missing"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_catalog_is_a_load_error() {
        let dir = std::env::temp_dir().join(format!("cynic-cap-i-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let catalog = dir.join("catalog.json");
        let installed = dir.join("installed.json");
        write(
            &catalog,
            r#"{"schema":"cynic.capability_catalog.v1","capabilities":[]}"#,
        );
        write(&installed, &installed_json(r#""node-control""#));

        let error = CapabilityView::load(&catalog, &installed).expect_err("must fail");
        assert!(error.0.contains("catalog_empty"));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Bilateral proof for the B2 host-operation capabilities: the repository
    /// catalog loads, every new capability admits with well-typed params, and
    /// every rendered argv targets the single `cynic host` surface.
    #[test]
    fn repo_catalog_admits_host_capabilities() {
        let root =
            std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(".."))
                .expect("workspace root must resolve");
        let catalog_path = root.join("infra/capabilities/catalog.json");
        let installed_path = root.join("infra/capabilities/catalog.installed.json");
        let installed_contents;
        if installed_path.exists() {
            installed_contents = std::fs::read_to_string(&installed_path).expect("read installed");
        } else {
            installed_contents = installed_json(
                "\"node-control\",\"bridge-install\",\"pve-helper-install\",\"principal-prepare\",\"runtime-seed-reconcile\",\"service-reconcile\",\"network-actuate\",\"network-baseline-apply\",\"storage-identity-enroll\",\"storage-schema-migrate\",\"hardware-baseline-enroll\",\"ssh-trust-admit\"",
            );
        }
        let dir = std::env::temp_dir().join(format!("cynic-cap-repo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let installed = dir.join("installed.json");
        write(&installed, &installed_contents);

        let view = CapabilityView::load(&catalog_path, &installed).expect("repo catalog must load");
        assert!(
            view.catalog.capabilities.len() >= 27,
            "unexpected capability count: {}",
            view.catalog.capabilities.len()
        );

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let registry = root.join("infra/registry.json");
        let manifest_str = manifest.display().to_string();
        let registry_str = registry.display().to_string();
        let hex64 = "a".repeat(64);
        let cases: Vec<(&str, Vec<(String, String)>)> = vec![
            ("bridge-install", vec![]),
            (
                "pve-helper-install",
                vec![
                    ("policy".to_string(), manifest_str.clone()),
                    ("bundle_dir".to_string(), manifest_str.clone()),
                ],
            ),
            ("principal-prepare", vec![]),
            (
                "runtime-seed-reconcile",
                vec![("request".to_string(), registry_str.clone())],
            ),
            (
                "service-reconcile",
                vec![
                    ("node".to_string(), "cortex".to_string()),
                    ("registry".to_string(), registry_str.clone()),
                    ("unit".to_string(), "cynic-node.service".to_string()),
                ],
            ),
            (
                "network-actuate",
                vec![
                    ("profile".to_string(), "service-route".to_string()),
                    ("target".to_string(), "cortex".to_string()),
                    ("registry".to_string(), registry_str.clone()),
                ],
            ),
            (
                "network-baseline-apply",
                vec![
                    ("profile".to_string(), "service-route".to_string()),
                    ("target".to_string(), "cortex".to_string()),
                    ("registry".to_string(), registry_str.clone()),
                ],
            ),
            (
                "storage-identity-enroll",
                vec![
                    ("endpoint".to_string(), "cortex:8006".to_string()),
                    ("namespace".to_string(), "cynic".to_string()),
                    ("database".to_string(), "cynic".to_string()),
                    ("admin_user".to_string(), "root".to_string()),
                    ("username".to_string(), "cynic".to_string()),
                ],
            ),
            (
                "storage-schema-migrate",
                vec![
                    ("endpoint".to_string(), "cortex:8006".to_string()),
                    ("namespace".to_string(), "cynic".to_string()),
                    ("database".to_string(), "cynic".to_string()),
                    ("admin_user".to_string(), "root".to_string()),
                    ("schema_file".to_string(), manifest_str.clone()),
                ],
            ),
            ("hardware-baseline-enroll", vec![]),
            (
                "ssh-trust-admit",
                vec![
                    ("host".to_string(), "cortex".to_string()),
                    ("fingerprint".to_string(), hex64),
                    ("attestation_source".to_string(), "direct-ssh".to_string()),
                ],
            ),
        ];
        assert_eq!(cases.len(), 11, "one case per new capability");
        for (id, params) in &cases {
            match view.admit(id, params) {
                CapabilityDecision::Admitted {
                    capability_id,
                    argv,
                    requires_root,
                    ..
                } => {
                    assert_eq!(&capability_id, id);
                    assert!(requires_root, "{id} must require root");
                    assert!(
                        argv.len() >= 3 && argv[0] == "/usr/local/bin/cynic" && argv[1] == "host",
                        "{id} must render through the single binary: {argv:?}"
                    );
                }
                CapabilityDecision::Blocked { blockers } => {
                    panic!("{id} must admit, blocked by: {blockers:?}")
                }
            }
        }

        let rules = view.catalog.sudoers_rules("cynic", "/usr/local/bin/cynic");
        for (id, _) in &cases {
            assert!(
                rules.iter().any(|rule| rule.contains(id)),
                "sudoers membrane must cover {id}"
            );
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
