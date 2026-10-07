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

/// Repository-relative default catalog.
pub const DEFAULT_CATALOG_PATH: &str = "infra/capabilities/catalog.json";

/// Node-local installed set, written by `cynic install`.
pub const DEFAULT_INSTALLED_PATH: &str = "/etc/cynic/capabilities.installed.json";

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

/// Resolve the catalog path, honouring the environment override used by the
/// installer and the tests. Never guesses: an unset variable yields the
/// repository-relative default.
pub fn catalog_path_from_env() -> PathBuf {
    std::env::var_os("CYNIC_CAPABILITY_CATALOG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CATALOG_PATH))
}

/// Resolve the installed-set path.
pub fn installed_path_from_env() -> PathBuf {
    std::env::var_os("CYNIC_CAPABILITY_INSTALLED")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_INSTALLED_PATH))
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
}
