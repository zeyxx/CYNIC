//! Declarative bounded-helper policy for the Proxmox host (`cynic-host-proxmox`).
//!
//! The capability list is DATA, not code: adding an incremental sudo
//! capability for the `cynic` principal means editing the policy JSON and
//! re-projecting the bundle — never editing a shell script. The executor is
//! the compiled `pct-bounded-exec` binary (a thin allowlisted argv wrapper
//! around `/usr/sbin/pct`), so every granted argv is verifiable against the
//! helper's own dispatch table and the artifact is digest-pinnable.
//!
//! Authority chain (see skill cynic-privileged-execution):
//! HumanAdmin installs the bundle once; RuntimeMachineIdentity then calls the
//! exact allowed argv through `sudo -n`; the sudoers file is a generated
//! projection of this policy, never hand-written.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::bounded_helper::BoundedHelperProjection;
use crate::command_surface::HostCommandSurface;
use crate::install_contract::{MODE_0440, MODE_0755, root_owned_path};

pub const PVE_HELPER_POLICY_SCHEMA: &str = "cynic.host.pve_helper_policy.v1";
pub const PVE_HELPER_BUNDLE_MANIFEST_SCHEMA: &str = "cynic.host.pve_helper_bundle_manifest.v1";

pub const DEFAULT_POLICY_PATH: &str = "infra/privilege/pve-pct-helper.policy.json";

/// One incremental sudo capability: an exact argv prefix the `cynic`
/// principal may run as root via the bounded executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PveHelperCapability {
    /// Stable capability id used in audits and journal lines.
    pub id: String,
    /// Executor subcommand (the helper's own allowlist key).
    pub action: String,
    /// Fixed trailing arguments after `<action>` (empty for bare actions).
    #[serde(default)]
    pub args: Vec<String>,
    /// Why this capability exists; shows up in the sudoers projection header.
    pub purpose: String,
}

impl PveHelperCapability {
    /// Full argv relative to the helper binary.
    pub fn argv(&self) -> Vec<String> {
        let mut argv = vec![self.action.clone()];
        argv.extend(self.args.iter().cloned());
        argv
    }
}

/// Root-owned executor + generated sudoers membrane for one subject on the
/// Proxmox host. Serde-shaped so the policy can live in `infra/privilege/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PveHelperPolicy {
    pub schema: String,
    /// Sudoers subject (principal) — always `cynic` in CYNIC runtime law.
    pub subject: String,
    /// Absolute helper path on the hypervisor.
    pub helper_path: String,
    /// Absolute generated-sudoers path on the hypervisor.
    pub sudoers_path: String,
    /// Hypervisor-side pct binary the executor wraps.
    pub pct_path: String,
    /// Incremental capability set — extend this list to grant more.
    pub capabilities: Vec<PveHelperCapability>,
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
        if self.capabilities.is_empty() {
            return Err(PvePolicyError(
                "policy grants zero capabilities — refuse to project an empty membrane".into(),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for capability in &self.capabilities {
            if !is_valid_token(&capability.id) || !is_valid_token(&capability.action) {
                return Err(PvePolicyError(format!(
                    "capability id/action must match ^[a-z0-9-]{{1,48}}$: ({}, {})",
                    capability.id, capability.action
                )));
            }
            if !seen.insert(capability.id.clone()) {
                return Err(PvePolicyError(format!(
                    "duplicate capability id {}",
                    capability.id
                )));
            }
            for arg in &capability.args {
                if arg.is_empty() || arg.contains('\0') || arg.contains('\n') {
                    return Err(PvePolicyError(format!(
                        "capability {} has an invalid fixed arg",
                        capability.id
                    )));
                }
            }
        }
        Ok(())
    }

    /// The typed projection consumed by the existing bundle/install machinery.
    pub fn projection(&self) -> BoundedHelperProjection {
        BoundedHelperProjection::new(
            self.subject.clone(),
            self.helper_path.clone(),
            self.sudoers_path.clone(),
            "Generated from infra/privilege/pve-pct-helper.policy.json (incremental \
             capabilities for the cynic principal on the Proxmox host).",
        )
        .with_allowed_invocations(
            self.capabilities
                .iter()
                .map(|capability| {
                    let mut argv = vec![self.helper_path.clone()];
                    argv.extend(capability.argv());
                    argv
                })
                .collect(),
        )
    }

    /// The executor source ships beside the policy; digest goes into the manifest.
    pub fn executor_source_candidates(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("infra/host-bootstrap/pve/pct-bounded-exec.sh"),
            PathBuf::from("/usr/local/share/cynic/pct-bounded-exec.sh"),
        ]
    }
}

/// Install-time command surfaces (root-owned artifacts), mirroring the
/// bridge bundle contract modes: helper 0755, sudoers 0440.
pub fn pve_helper_install_commands(
    policy: &PveHelperPolicy,
    staged_dir: &Path,
) -> Vec<HostCommandSurface> {
    let helper_source = staged_dir.join("pct-bounded-exec");
    let sudoers_source = staged_dir.join("pve-pct.sudoers");
    vec![
        crate::command_surface::install_file_command(
            "install_helper",
            helper_source.to_string_lossy().as_ref(),
            &policy.helper_path,
            root_owned_path(MODE_0755),
        ),
        crate::command_surface::install_file_command(
            "install_sudoers",
            sudoers_source.to_string_lossy().as_ref(),
            &policy.sudoers_path,
            root_owned_path(MODE_0440),
        ),
        crate::command_surface::validate_sudoers_command(
            "validate_sudoers",
            policy.sudoers_path.clone(),
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
/// installer's exit code): deny-test + allow-probe + artifact modes.
pub fn pve_helper_read_back_commands(policy: &PveHelperPolicy) -> Vec<HostCommandSurface> {
    let mut commands = vec![crate::command_surface::argv_command(
        "prove_denied_action",
        vec![
            "sudo".to_string(),
            "-n".to_string(),
            "-l".to_string(),
            format!("{} exec", policy.helper_path),
        ],
    )];
    for capability in &policy.capabilities {
        let mut probe = vec![
            "sudo".to_string(),
            "-n".to_string(),
            "-l".to_string(),
            policy.helper_path.clone(),
        ];
        probe.extend(capability.argv());
        let role: &'static str =
            Box::leak(format!("prove_allowed_{}", capability.id).into_boxed_str());
        commands.push(crate::command_surface::argv_command(role, probe));
    }
    commands.push(crate::command_surface::argv_command(
        "observe_helper_mode",
        vec![
            "stat".to_string(),
            "-c".to_string(),
            "%U:%G %a".to_string(),
            policy.helper_path.clone(),
        ],
    ));
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
    pub subject: String,
    pub helper_target: String,
    pub sudoers_target: String,
    pub capability_ids: Vec<String>,
    pub executor_sha256: Option<String>,
}

pub fn pve_helper_bundle_manifest(
    policy: &PveHelperPolicy,
    executor_source: &Path,
) -> PveHelperBundleManifest {
    let executor_sha256 = std::fs::read(executor_source)
        .ok()
        .map(|bytes| use_sha2_hex(&bytes));
    PveHelperBundleManifest {
        schema: PVE_HELPER_BUNDLE_MANIFEST_SCHEMA,
        policy_schema: policy.schema.clone(),
        subject: policy.subject.clone(),
        helper_target: policy.helper_path.clone(),
        sudoers_target: policy.sudoers_path.clone(),
        capability_ids: policy
            .capabilities
            .iter()
            .map(|capability| capability.id.clone())
            .collect(),
        executor_sha256,
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
            "helper_path": "/usr/local/sbin/pct-bounded-exec",
            "sudoers_path": "/etc/sudoers.d/cynic-pve-pct",
            "pct_path": "/usr/sbin/pct",
            "capabilities": [
                {"id": "pct-list", "action": "list", "args": [], "purpose": "CT inventory"},
                {"id": "pct-probe", "action": "probe", "args": [],
                 "purpose": "read-only git CT observation"},
                {"id": "forgejo-token", "action": "generate-token", "args": ["--scopes", "write:repository"],
                 "purpose": "one-shot machine token issuance"}
            ]
        }"#,
        )
        .expect("sample policy parses")
    }

    #[test]
    fn sample_policy_validates_and_projects_sudoers() {
        let policy = sample_policy();
        policy.validate().expect("valid");
        let projection = policy.projection();
        assert_eq!(projection.subject, "cynic");
        let rule = projection.sudoers_rule();
        for fragment in [
            "/usr/local/sbin/pct-bounded-exec list",
            "/usr/local/sbin/pct-bounded-exec probe",
            "/usr/local/sbin/pct-bounded-exec generate-token --scopes write:repository",
        ] {
            assert!(rule.contains(fragment), "missing {fragment} in {rule}");
        }
        // No wildcard: every line is an exact argv ending in a fixed token.
        assert!(!rule.contains('*'), "sudoers must not contain wildcards");
    }

    #[test]
    fn rejects_wrong_subject_and_empty_capabilities() {
        let mut policy = sample_policy();
        policy.subject = "root".into();
        assert!(policy.validate().is_err());
        policy.subject = "cynic".into();
        policy.capabilities.clear();
        assert!(policy.validate().is_err());
    }

    #[test]
    fn rejects_bad_tokens_and_duplicate_ids() {
        let mut policy = sample_policy();
        policy.capabilities[0].id = "Bad_Id".into();
        assert!(policy.validate().is_err());
        policy.capabilities[0].id = "pct-list".into();
        policy.capabilities[2].id = "pct-list".into();
        assert!(policy.validate().is_err());
        policy.capabilities[2].id = "forgejo-token".into();
        policy.validate().expect("back to valid");
    }

    #[test]
    fn read_back_covers_every_capability_plus_denial_probe() {
        let policy = sample_policy();
        let read_back = pve_helper_read_back_commands(&policy);
        let denial = read_back
            .iter()
            .find(|surface| surface.role == "prove_denied_action")
            .expect("denial probe present");
        // The denial proof lists sudo grants for the helper; an action outside
        // the policy must not appear as a separate granted token.
        let denial_argv = invocation_argv(denial);
        assert!(
            denial_argv
                .iter()
                .any(|part| part.ends_with("pct-bounded-exec exec"))
        );
        assert!(!denial_argv.contains(&"list".to_string()));
        assert!(!denial_argv.contains(&"probe".to_string()));
        for capability in &policy.capabilities {
            let expected_role = format!("prove_allowed_{}", capability.id);
            assert!(
                read_back.iter().any(|surface| {
                    surface.role == expected_role
                        && invocation_argv(surface).ends_with(capability.argv().as_slice())
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
    fn install_commands_pin_root_owned_modes() {
        let policy = sample_policy();
        let staged = Path::new("/tmp/stage");
        let commands = pve_helper_install_commands(&policy, staged);
        assert!(
            commands
                .iter()
                .any(|surface| surface.role == "install_helper"),
            "helper install command missing"
        );
        assert!(
            commands
                .iter()
                .any(|surface| surface.role == "install_sudoers"),
            "sudoers install command missing"
        );
        assert!(
            commands
                .iter()
                .any(|surface| surface.role == "validate_sudoers"),
            "visudo validation missing"
        );
        // Modes are carried by the typed InstallFile policies, not strings:
        for surface in &commands {
            if let crate::command_surface::HostCommandInvocation::InstallFile {
                policy: file_policy,
                ..
            } = &surface.invocation
            {
                assert_eq!(file_policy.owner.user, "root");
                assert!(file_policy.mode.octal == "0755" || file_policy.mode.octal == "0440");
            }
        }
    }
}
