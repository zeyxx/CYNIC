//! Deploy location registry — typed binding for `infra/deploy-locations.json`.
//!
//! Single Source of Truth for WHERE the cynic binary lives on each fleet
//! machine and HOW it is deployed there. The file replaced the previous
//! binary-locations registry, which declared one path, one sha256 and one
//! status per machine while every machine actually carried TWO binaries in
//! different profiles — and whose declared sha256 matched no live binary
//! anywhere (measured 2026-10-19).
//!
//! Zero I/O here: parsing is the caller's job (CLI adapter), this module
//! decides and validates. `cynic-release` stays a pure decision engine.

use serde::{Deserialize, Serialize};

/// Schema name of the deploy location registry.
pub const DEPLOY_LOCATIONS_SCHEMA: &str = "cynic.deploy_locations.v1";

/// How the live binary is swapped on a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployLayout {
    /// `releases/<tag>` + `current` symlink; the swap is a symlink rename.
    Symlink,
    /// A single live file; the swap is an atomic rename over it.
    Flat,
}

/// Read-back commands proving the deployed binary is the deployed commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployReadback {
    /// Command proving the bytes on disk.
    pub sha256_command: String,
    /// Command proving the baked source identity (the commit).
    pub identity_command: String,
    /// Command proving the binary works (smoke through the real path).
    pub smoke_command: String,
    /// Expected smoke outcome.
    pub smoke_expect: String,
}

/// One machine's deploy target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployMachine {
    pub hostname: String,
    /// Node id in `infra/registry.json` — the fleet reference, never a hostname.
    pub registry_node_id: String,
    pub os: String,
    /// SSH target; `local` for the machine the operator runs on.
    pub ssh_target: String,
    /// Secondary SSH form tried when the primary does not resolve.
    #[serde(default)]
    pub ssh_target_fallback: Option<String>,
    /// Principal that owns the deploy path.
    pub principal: String,
    /// Deploy path, using the registry's expansion vocabulary.
    pub deploy_path: String,
    /// The same path, resolved for the standard deployment (state root on a
    /// deployed node, operator home on a workstation). Both the template and
    /// its standard resolution are declared so the executor never invents a
    /// third spelling; the runtime resolution uses
    /// [`resolve_deploy_path`].
    pub deploy_path_resolved: String,
    /// `owner:group` of the deploy path.
    pub path_owner: String,
    pub layout: DeployLayout,
    /// Backup file name pattern for the live binary.
    pub backup_pattern: String,
    pub readback: DeployReadback,
    /// Measured state at the observation date.
    #[serde(default)]
    pub measured: Option<MeasuredDeployment>,
}

/// A binary that exists on the machine but is NOT the deploy target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtherBinary {
    pub path: String,
    pub sha256: String,
    pub git_sha: String,
    pub profile: String,
    pub owner: String,
    /// Whether the deploy principal can write this path.
    pub writable_by_principal: bool,
    /// What to do with it (measured judgement, not a wish).
    pub disposition: String,
}

/// Measured truth about one machine's binaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasuredDeployment {
    pub observed_at: String,
    pub deploy_path_sha256: String,
    pub deploy_path_git_sha: String,
    pub deploy_path_profile: String,
    #[serde(default)]
    pub other_binary: Option<OtherBinary>,
}

/// The whole registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployLocationsContract {
    pub schema: String,
    pub status: String,
    pub machines: Vec<DeployMachine>,
}

impl DeployLocationsContract {
    pub fn parse(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Look a machine up by hostname, registry node id, or SSH target.
    pub fn machine(&self, key: &str) -> Option<&DeployMachine> {
        self.machines.iter().find(|machine| {
            machine.hostname == key
                || machine.registry_node_id == key
                || machine.ssh_target == key
                || machine
                    .ssh_target_fallback
                    .as_deref()
                    .map(|fallback| fallback == key)
                    .unwrap_or(false)
        })
    }

    /// Machine hostnames in declaration order — the canary order when the
    /// policy does not override it.
    pub fn hostnames(&self) -> Vec<String> {
        self.machines
            .iter()
            .map(|machine| machine.hostname.clone())
            .collect()
    }
}

/// A refused deploy target: named reason, never a silent fallback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployTargetRefusal {
    pub machine: String,
    pub reason: String,
}

/// Decide whether a machine can be deployed to, and why not when it cannot.
///
/// Refusals are explicit and typed, because a deploy that guesses a path
/// writes to the wrong place on a fleet.
pub fn decide_deploy_target(
    locations: &DeployLocationsContract,
    machine: &str,
) -> Result<DeployMachine, DeployTargetRefusal> {
    let found = locations.machine(machine).ok_or_else(|| DeployTargetRefusal {
        machine: machine.to_string(),
        reason: format!("machine {machine} is not declared in the deploy locations registry"),
    })?;
    if found.deploy_path_resolved.trim().is_empty() {
        return Err(DeployTargetRefusal {
            machine: machine.to_string(),
            reason: "deploy path is empty".to_string(),
        });
    }
    if !found.deploy_path.starts_with("${") && !found.deploy_path.starts_with('/') {
        return Err(DeployTargetRefusal {
            machine: machine.to_string(),
            reason: format!(
                "deploy path {:?} is neither an absolute path nor a registry expansion",
                found.deploy_path
            ),
        });
    }
    if found.readback.smoke_command.trim().is_empty() {
        return Err(DeployTargetRefusal {
            machine: machine.to_string(),
            reason: "no smoke command declared: a deploy without a read-back is not a deploy"
                .to_string(),
        });
    }
    if found.readback.identity_command.trim().is_empty() {
        return Err(DeployTargetRefusal {
            machine: machine.to_string(),
            reason: "no identity command declared: the baked commit would stay unproven"
                .to_string(),
        });
    }
    Ok(found.clone())
}

/// Whether the deployed binary is the deployed commit, judged on the identity
/// surface's own report. The sha256 alone does not prove the commit (same
/// bytes can be built from a different tree only in theory — but the baked
/// SHA is the surface that answers the question, so it is the one read).
pub fn identity_matches(
    observed_git_sha: &str,
    deployed_commit: &str,
) -> bool {
    let observed = observed_git_sha.trim();
    let expected = deployed_commit.trim();
    !observed.is_empty() && !expected.is_empty() && observed == expected
}

/// Resolve a declared deploy path template against the machine's roots.
///
/// The templates use the vocabulary `infra/directory-tree.json` declares:
/// `${state_root}` (the cynic principal home on a deployed node) and
/// `${home}` (the principal's home). Resolution is explicit: the caller
/// supplies the roots, so a missing root is a refusal, never a guess.
pub fn resolve_deploy_path(
    template: &str,
    state_root: Option<&str>,
    home: Option<&str>,
) -> Result<String, DeployTargetRefusal> {
    let mut out = template.trim().to_string();
    if out.contains("${state_root}") {
        let root = state_root
            .map(|value| value.trim().trim_end_matches('/'))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| DeployTargetRefusal {
                machine: template.to_string(),
                reason: "deploy path needs ${state_root} but no state root was resolved".to_string(),
            })?;
        out = out.replace("${state_root}", root);
    }
    if out.contains("${home}") {
        let home = home
            .map(|value| value.trim().trim_end_matches('/'))
            .filter(|value| !value.is_empty())
            .ok_or_else(|| DeployTargetRefusal {
                machine: template.to_string(),
                reason: "deploy path needs ${home} but no home was resolved".to_string(),
            })?;
        out = out.replace("${home}", home);
    }
    if out.contains("${") {
        return Err(DeployTargetRefusal {
            machine: template.to_string(),
            reason: format!("deploy path {out} carries an unresolved expansion"),
        });
    }
    if !out.starts_with('/') {
        return Err(DeployTargetRefusal {
            machine: template.to_string(),
            reason: format!("resolved deploy path {out} is not absolute"),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> DeployLocationsContract {
        DeployLocationsContract {
            schema: DEPLOY_LOCATIONS_SCHEMA.to_string(),
            status: "normative".to_string(),
            machines: vec![
                DeployMachine {
                    hostname: "builder".to_string(),
                    registry_node_id: "cynic-builder".to_string(),
                    os: "Ubuntu".to_string(),
                    ssh_target: "cynic-builder-ssh".to_string(),
                    ssh_target_fallback: None,
                    principal: "cynic".to_string(),
                    deploy_path: "${state_root}/bin/cynic".to_string(),
                    deploy_path_resolved: "/var/lib/cynic/bin/cynic".to_string(),
                    path_owner: "cynic:cynic".to_string(),
                    layout: DeployLayout::Symlink,
                    backup_pattern: "binary.bak-<sha>".to_string(),
                    readback: DeployReadback {
                        sha256_command: "sha256sum /var/lib/cynic/bin/cynic".to_string(),
                        identity_command: "/var/lib/cynic/bin/cynic version --json".to_string(),
                        smoke_command: "/var/lib/cynic/bin/cynic source status --json".to_string(),
                        smoke_expect: "exit code 0".to_string(),
                    },
                    measured: None,
                },
                DeployMachine {
                    hostname: "cortex".to_string(),
                    registry_node_id: "cynic-host-cortex".to_string(),
                    os: "Ubuntu".to_string(),
                    ssh_target: "local".to_string(),
                    ssh_target_fallback: None,
                    principal: "user".to_string(),
                    deploy_path: "${home}/.local/bin/cynic".to_string(),
                    deploy_path_resolved: "/home/user/.local/bin/cynic".to_string(),
                    path_owner: "user:user".to_string(),
                    layout: DeployLayout::Flat,
                    backup_pattern: "cynic.bak-<sha>".to_string(),
                    readback: DeployReadback {
                        sha256_command: "sha256sum /home/user/.local/bin/cynic".to_string(),
                        identity_command: "/home/user/.local/bin/cynic version --json".to_string(),
                        smoke_command: "/home/user/.local/bin/cynic source status --json".to_string(),
                        smoke_expect: "exit code 0".to_string(),
                    },
                    measured: None,
                },
            ],
        }
    }

    #[test]
    fn machine_is_found_by_any_of_its_names() {
        let locations = contract();
        assert_eq!(
            locations.machine("builder").map(|m| m.hostname.as_str()),
            Some("builder")
        );
        assert_eq!(
            locations
                .machine("cynic-builder")
                .map(|m| m.hostname.as_str()),
            Some("builder")
        );
        assert!(locations.machine("ghost").is_none());
    }

    #[test]
    fn unknown_machine_is_refused_with_a_named_reason() {
        let refusal = decide_deploy_target(&contract(), "ghost").expect_err("must refuse");
        assert!(refusal.reason.contains("not declared"));
    }

    #[test]
    fn a_machine_without_readback_is_refused() {
        let mut locations = contract();
        locations.machines[0].readback.smoke_command = String::new();
        let refusal = decide_deploy_target(&locations, "builder").expect_err("must refuse");
        assert!(refusal.reason.contains("smoke"));
    }

    #[test]
    fn a_machine_without_identity_surface_is_refused() {
        let mut locations = contract();
        locations.machines[0].readback.identity_command = String::new();
        let refusal = decide_deploy_target(&locations, "builder").expect_err("must refuse");
        assert!(refusal.reason.contains("identity"));
    }

    #[test]
    fn a_relative_path_is_refused() {
        let mut locations = contract();
        locations.machines[0].deploy_path = "bin/cynic".to_string();
        let refusal = decide_deploy_target(&locations, "builder").expect_err("must refuse");
        assert!(refusal.reason.contains("absolute"));
    }

    #[test]
    fn identity_matches_only_on_the_exact_commit() {
        assert!(identity_matches("abc123", "abc123"));
        assert!(!identity_matches("abc123", "abc124"));
        assert!(!identity_matches("", "abc123"));
        assert!(!identity_matches("abc123", ""));
    }

    #[test]
    fn hostnames_follow_declaration_order() {
        assert_eq!(contract().hostnames(), vec!["builder", "cortex"]);
    }

    #[test]
    fn deploy_path_resolves_against_the_machine_roots() {
        let resolved = resolve_deploy_path(
            "${state_root}/bin/cynic",
            Some("/var/lib/cynic"),
            Some("/home/operator"),
        )
        .expect("resolve");
        assert_eq!(resolved, "/var/lib/cynic/bin/cynic");
    }

    #[test]
    fn a_missing_root_is_refused_not_guessed() {
        let refusal = resolve_deploy_path("${state_root}/bin/cynic", None, None)
            .expect_err("must refuse");
        assert!(refusal.reason.contains("state root"));
        let refusal = resolve_deploy_path("${home}/.local/bin/cynic", Some("/var/lib/cynic"), None)
            .expect_err("must refuse");
        assert!(refusal.reason.contains("home"));
    }

    #[test]
    fn an_unresolved_expansion_is_refused() {
        let refusal = resolve_deploy_path("${unknown}/bin/cynic", Some("/var/lib/cynic"), None)
            .expect_err("must refuse");
        assert!(refusal.reason.contains("unresolved expansion"));
    }

    #[test]
    fn a_relative_resolved_path_is_refused() {
        let refusal = resolve_deploy_path("bin/cynic", Some("/var/lib/cynic"), None)
            .expect_err("must refuse");
        assert!(refusal.reason.contains("not absolute"));
    }
}
