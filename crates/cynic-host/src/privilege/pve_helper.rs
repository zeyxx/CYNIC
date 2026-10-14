//! Declarative bounded-helper policy for the Proxmox host (`cynic-host-proxmox`).
//!
//! The capability list is DATA, and there is now ONE data source:
//! `infra/capabilities/catalog.json` (`CapabilityCatalog`). This module used to
//! carry a second schema (`PveHelperCapability { id, action, args }`) beside the
//! catalog's `Capability { id, argv_template, params }` — two representations of
//! one concept, with divergent capability ids (`probe` vs `pct-probe`) and two
//! sudoers generators. That duplicate is gone.
//!
//! This module keeps only what is genuinely install-site specific: where the
//! binary lands, where the sudoers membrane lands, and the HumanAdmin bootstrap
//! choreography. The capability set itself is the catalog's.
//!
//! Authority chain (see skill cynic-privileged-execution):
//! HumanAdmin installs once; RuntimeMachineIdentity then calls the exact
//! allowed argv through `sudo -n`; the sudoers file is a generated projection
//! of the catalog, never hand-written.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::command_surface::HostCommandSurface;
use crate::install_contract::{MODE_0440, root_owned_path};
use cynic_protocol::privilege::CapabilityCatalog;

pub const PVE_HELPER_POLICY_SCHEMA: &str = "cynic.host.pve_helper_policy.v1";
pub const PVE_HELPER_BUNDLE_MANIFEST_SCHEMA: &str = "cynic.host.pve_helper_bundle_manifest.v1";

/// The unified catalog path is owned by `capability_catalog`; re-exported here
/// so pve_helper callers do not need a second constant for one concept.
pub use crate::privilege::capability_catalog::DEFAULT_CATALOG_PATH;
/// The legacy install-site policy (paths only; capabilities come from the catalog).
pub const DEFAULT_POLICY_PATH: &str = "infra/privilege/pve-pct-helper.policy.json";

/// Install-site facts for the bounded membrane on the Proxmox host.
///
/// NO capability list here. The capabilities live in the catalog; this struct
/// answers only "where do the artifacts land".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PveHelperPolicy {
    pub schema: String,
    /// Sudoers subject (principal) — always `cynic` in CYNIC runtime law.
    pub subject: String,
    /// Absolute binary path on the hypervisor. In the single-binary design this
    /// is the `cynic` binary itself, which IS the bounded executor.
    pub helper_path: String,
    /// Absolute generated-sudoers path on the hypervisor.
    pub sudoers_path: String,
    /// Hypervisor-side pct binary the capabilities wrap.
    pub pct_path: String,
    /// Path to the unified capability catalog. Defaults to DEFAULT_CATALOG_PATH.
    #[serde(default = "default_catalog_path")]
    pub catalog_path: String,
}

fn default_catalog_path() -> String {
    DEFAULT_CATALOG_PATH.to_string()
}

impl PveHelperPolicy {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PvePolicyError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path)
            .map_err(|error| PvePolicyError(format!("read {}: {error}", path.display())))?;
        let policy: Self = serde_json::from_str(&raw)
            .map_err(|error| PvePolicyError(format!("parse {}: {error}", path.display())))?;
        policy.validate()?;
        Ok(policy)
    }

    /// Load the install-site policy AND the capability catalog it points at.
    ///
    /// Both are needed to project a membrane: the policy says where, the
    /// catalog says what. A missing or invalid catalog is an error, never an
    /// empty (permissive-looking) membrane.
    pub fn load_with_catalog(
        policy_path: impl AsRef<Path>,
    ) -> Result<(Self, CapabilityCatalog), PvePolicyError> {
        let policy = Self::load(policy_path)?;
        let raw = std::fs::read_to_string(&policy.catalog_path).map_err(|error| {
            PvePolicyError(format!("read catalog {}: {error}", policy.catalog_path))
        })?;
        let catalog: CapabilityCatalog = serde_json::from_str(&raw).map_err(|error| {
            PvePolicyError(format!("parse catalog {}: {error}", policy.catalog_path))
        })?;
        let blockers = catalog.validate();
        if !blockers.is_empty() {
            return Err(PvePolicyError(format!(
                "catalog {} is invalid: {}",
                policy.catalog_path,
                blockers.join(", ")
            )));
        }
        Ok((policy, catalog))
    }

    pub fn validate(&self) -> Result<(), PvePolicyError> {
        if self.schema != PVE_HELPER_POLICY_SCHEMA {
            return Err(PvePolicyError(format!(
                "schema mismatch: expected {PVE_HELPER_POLICY_SCHEMA}, got {}",
                self.schema
            )));
        }
        if self.subject != "cynic" {
            return Err(PvePolicyError(format!(
                "subject must be the cynic principal, got {:?}",
                self.subject
            )));
        }
        for (field, value) in [
            ("helper_path", &self.helper_path),
            ("sudoers_path", &self.sudoers_path),
            ("pct_path", &self.pct_path),
            ("catalog_path", &self.catalog_path),
        ] {
            if !value.starts_with('/') && field != "catalog_path" {
                return Err(PvePolicyError(format!(
                    "{field} must be an absolute path, got {value:?}"
                )));
            }
        }
        Ok(())
    }

    /// The generated sudoers membrane: one rule per `requires_root` capability
    /// in the catalog. Delegates to the catalog — this module does NOT render
    /// sudoers itself, because two renderers diverge silently (they did).
    pub fn sudoers_projection(&self, catalog: &CapabilityCatalog) -> String {
        let rules = catalog.sudoers_rules(&self.subject, &self.helper_path);
        if rules.is_empty() {
            return String::new();
        }
        format!(
            "# Generated from {} (capability catalog)\n# Subject: {}\n{}\n",
            self.catalog_path,
            self.subject,
            rules.join("\n")
        )
    }

    /// The exact argv a read-back probe must find granted for a capability.
    ///
    /// `sudo -n -l <argv>` checks a permission WITHOUT running it, so the probe
    /// uses the same argv shape the membrane declares: the binary, the
    /// `privilege exec <id>` arm, and the parameter names.
    pub fn probe_argv(&self, capability_id: &str) -> Vec<String> {
        vec![
            self.helper_path.clone(),
            "privilege".to_string(),
            "exec".to_string(),
            capability_id.to_string(),
        ]
    }
}

/// Install-time command surfaces (root-owned artifacts).
///
/// ORDER IS A SECURITY PROPERTY. `validate_sudoers` runs BEFORE
/// `install_sudoers`: a sudoers file that fails `visudo -c` must never land in
/// `/etc/sudoers.d/`, because an unparseable file there can break `sudo`
/// entirely — including the ability to fix it.
pub fn pve_helper_install_commands(
    policy: &PveHelperPolicy,
    staged_dir: &Path,
) -> Vec<HostCommandSurface> {
    let sudoers_source = staged_dir.join("pve-pct.sudoers");
    vec![
        // Validate the STAGED file, then install it. Never the other way.
        crate::command_surface::validate_sudoers_command(
            "validate_sudoers",
            sudoers_source.to_string_lossy().as_ref(),
        ),
        crate::command_surface::install_file_command(
            "install_sudoers",
            sudoers_source.to_string_lossy().as_ref(),
            &policy.sudoers_path,
            root_owned_path(MODE_0440),
        ),
        // PATH shim so non-interactive shells resolve pct.
        crate::command_surface::argv_command(
            "ensure_pct_on_path",
            vec![
                "ln".to_string(),
                "-sf".to_string(),
                policy.pct_path.clone(),
                "/usr/local/bin/pct".to_string(),
            ],
        ),
    ]
}

/// Post-install read-back through ordinary host observation (not just the
/// installer's exit code): deny-test + allow-probe per capability + modes.
pub fn pve_helper_read_back_commands(
    policy: &PveHelperPolicy,
    catalog: &CapabilityCatalog,
) -> Vec<HostCommandSurface> {
    let mut commands = vec![crate::command_surface::argv_command(
        "prove_denied_action",
        vec![
            "sudo".to_string(),
            "-n".to_string(),
            "-l".to_string(),
            format!("{} privilege exec", policy.helper_path),
        ],
    )];
    for capability in catalog.capabilities.iter().filter(|c| c.requires_root) {
        let mut probe = vec!["sudo".to_string(), "-n".to_string(), "-l".to_string()];
        probe.extend(policy.probe_argv(&capability.id));
        let role: &'static str =
            Box::leak(format!("prove_allowed_{}", capability.id).into_boxed_str());
        commands.push(crate::command_surface::argv_command(role, probe));
    }
    commands.push(crate::command_surface::argv_command(
        "observe_sudoers_mode",
        vec![
            "stat".to_string(),
            "-c".to_string(),
            "%U:%G %a".to_string(),
            policy.sudoers_path.clone(),
        ],
    ));
    commands
}

/// Machine-readable manifest written next to the staged bundle.
#[derive(Debug, Clone, Serialize)]
pub struct PveHelperBundleManifest {
    pub schema: &'static str,
    pub policy_schema: String,
    pub catalog_schema: String,
    pub subject: String,
    pub helper_target: String,
    pub sudoers_target: String,
    pub capability_ids: Vec<String>,
    pub sudoers_sha256: Option<String>,
}

pub fn pve_helper_bundle_manifest(
    policy: &PveHelperPolicy,
    catalog: &CapabilityCatalog,
    staged_sudoers: &Path,
) -> PveHelperBundleManifest {
    let sudoers_sha256 = std::fs::read(staged_sudoers)
        .ok()
        .map(|bytes| use_sha2_hex(&bytes));
    PveHelperBundleManifest {
        schema: PVE_HELPER_BUNDLE_MANIFEST_SCHEMA,
        policy_schema: policy.schema.clone(),
        catalog_schema: catalog.schema.clone(),
        subject: policy.subject.clone(),
        helper_target: policy.helper_path.clone(),
        sudoers_target: policy.sudoers_path.clone(),
        capability_ids: catalog
            .capabilities
            .iter()
            .filter(|c| c.requires_root)
            .map(|capability| capability.id.clone())
            .collect(),
        sudoers_sha256,
    }
}

fn use_sha2_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 48
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvePolicyError(pub String);

impl fmt::Display for PvePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "pve helper policy error: {}", self.0)
    }
}

impl std::error::Error for PvePolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_policy() -> PveHelperPolicy {
        serde_json::from_str(
            r#"{
            "schema": "cynic.host.pve_helper_policy.v1",
            "subject": "cynic",
            "helper_path": "/usr/local/bin/cynic",
            "sudoers_path": "/etc/sudoers.d/cynic",
            "pct_path": "/usr/sbin/pct"
        }"#,
        )
        .expect("sample policy parses")
    }

    fn sample_catalog() -> CapabilityCatalog {
        serde_json::from_str(
            r#"{
            "schema": "cynic.capability_catalog.v1",
            "capabilities": [
                {"id":"pct-list","domain":"hypervisor","argv_template":["/usr/sbin/pct","list"],"requires_root":true,"purpose":"inventory"},
                {"id":"pct-destroy","domain":"hypervisor","argv_template":["/usr/sbin/pct","destroy","{vmid}"],"params":[{"name":"vmid","kind":"vmid"}],"requires_root":true,"purpose":"destroy"},
                {"id":"node-control","domain":"runtime","argv_template":["node","control"],"requires_root":false,"purpose":"runtime"}
            ]
        }"#,
        )
        .expect("valid catalog")
    }

    #[test]
    fn policy_has_no_capability_list_of_its_own() {
        // Regression lock: the duplicate capability schema is gone. The policy
        // answers "where", the catalog answers "what".
        let policy = sample_policy();
        policy.validate().expect("valid");
        assert_eq!(policy.catalog_path, DEFAULT_CATALOG_PATH);
        // A policy carrying a `capabilities` key is no longer representable.
        let with_caps = r#"{"schema":"cynic.host.pve_helper_policy.v1","subject":"cynic",
            "helper_path":"/usr/local/bin/cynic","sudoers_path":"/etc/sudoers.d/cynic",
            "pct_path":"/usr/sbin/pct","capabilities":[]}"#;
        let parsed: PveHelperPolicy =
            serde_json::from_str(with_caps).expect("parses (ignores unknown)");
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn rejects_wrong_subject_and_relative_paths() {
        let mut policy = sample_policy();
        policy.subject = "root".into();
        assert!(policy.validate().is_err());
        policy.subject = "cynic".into();
        policy.helper_path = "relative/path".into();
        assert!(policy.validate().is_err(), "helper_path must be absolute");
        policy.helper_path = "/usr/local/bin/cynic".into();
        policy.validate().expect("back to valid");
    }

    #[test]
    fn sudoers_projection_delegates_to_the_catalog() {
        let policy = sample_policy();
        let catalog = sample_catalog();
        let projection = policy.sudoers_projection(&catalog);
        // Only requires_root capabilities appear.
        assert!(projection.contains("pct-list"));
        assert!(projection.contains("pct-destroy"));
        assert!(!projection.contains("node-control"));
        // Bounded class, not a bare wildcard.
        assert!(projection.contains("--param vmid=[0-9]*"));
        // Header names the single source.
        assert!(projection.contains("infra/capabilities/catalog.json"));
    }

    #[test]
    fn probe_argv_matches_the_declared_membrane_shape() {
        let policy = sample_policy();
        let argv = policy.probe_argv("pct-destroy");
        assert_eq!(
            argv,
            vec!["/usr/local/bin/cynic", "privilege", "exec", "pct-destroy"]
        );
    }

    #[test]
    fn read_back_covers_every_root_capability_plus_denial_probe() {
        let policy = sample_policy();
        let catalog = sample_catalog();
        let read_back = pve_helper_read_back_commands(&policy, &catalog);
        let denial = read_back
            .iter()
            .find(|surface| surface.role == "prove_denied_action")
            .expect("denial probe present");
        let denial_argv = invocation_argv(denial);
        assert!(
            denial_argv
                .iter()
                .any(|part| part.ends_with("cynic privilege exec")),
            "denial probe must name the exec arm: {denial_argv:?}"
        );
        for capability in catalog.capabilities.iter().filter(|c| c.requires_root) {
            let expected_role = format!("prove_allowed_{}", capability.id);
            assert!(
                read_back.iter().any(|surface| {
                    surface.role == expected_role
                        && invocation_argv(surface)
                            .ends_with(policy.probe_argv(&capability.id).as_slice())
                }),
                "missing allow-probe for {}",
                capability.id
            );
        }
    }

    fn invocation_argv(surface: &HostCommandSurface) -> Vec<String> {
        match &surface.invocation {
            crate::command_surface::HostCommandInvocation::Argv { argv } => argv.clone(),
            other => panic!("expected argv surface, got {other:?}"),
        }
    }

    #[test]
    fn install_validates_the_staged_sudoers_before_installing_it() {
        let policy = sample_policy();
        let staged = Path::new("/tmp/stage");
        let commands = pve_helper_install_commands(&policy, staged);
        let roles: Vec<&str> = commands.iter().map(|s| s.role.as_str()).collect();
        let validate_at = roles
            .iter()
            .position(|r| *r == "validate_sudoers")
            .expect("validate present");
        let install_at = roles
            .iter()
            .position(|r| *r == "install_sudoers")
            .expect("install present");
        assert!(
            validate_at < install_at,
            "validate must run BEFORE install, got order {roles:?}"
        );
        // Validation targets the STAGED file, never the live one.
        let validate = &commands[validate_at];
        let staged_path = match &validate.invocation {
            crate::command_surface::HostCommandInvocation::ValidateSudoers { path } => path.clone(),
            other => panic!("expected ValidateSudoers, got {other:?}"),
        };
        assert!(
            staged_path.contains("/tmp/stage"),
            "must validate the staged path: {staged_path}"
        );
        assert!(
            !staged_path.contains("/etc/sudoers.d/"),
            "must NOT validate the installed path: {staged_path}"
        );
    }

    #[test]
    fn install_commands_pin_root_owned_modes() {
        let policy = sample_policy();
        let staged = Path::new("/tmp/stage");
        let commands = pve_helper_install_commands(&policy, staged);
        for surface in &commands {
            if let crate::command_surface::HostCommandInvocation::InstallFile {
                policy: file_policy,
                ..
            } = &surface.invocation
            {
                assert_eq!(file_policy.owner.user, "root");
                assert_eq!(file_policy.mode.octal, "0440");
            }
        }
    }
}
