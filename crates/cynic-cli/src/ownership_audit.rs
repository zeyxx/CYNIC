use serde::Serialize;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::rust_source::{relative_path, rust_files_under, workspace_root};

const PROJECT_ROOT_TOKEN: &str = "CYNIC_PROJECT_ROOT";

const OWNERSHIP_SURFACES: &[OwnershipSurfaceSpec] = &[
    OwnershipSurfaceSpec {
        surface: "runtime_authority_bootstrap",
        crate_name: "cynic-host",
        owner: "cynic-host",
        runtime_principal: true,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic", "/var/log/cynic"],
        allowed_writers: &["root:root", "cynic:cynic"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/runtime_layout.rs",
            "src/domain/principal.rs",
            "src/domain/packages.rs",
            "src/framework_bootstrap/",
            "src/registry.rs",
            "src/repo_access.rs",
            "src/principal_prepare.rs",
            "src/lifecycle/",
            "src/runtime_authority.rs",
        ],
        must_not_depend_on_project_root: false,
        migration_target: None,
        anti_patterns: &[
            "runtime authority logic should stay near host, not kernel",
            "bootstrap and principal paths must remain explicit and root-owned where needed",
        ],
    },
    OwnershipSurfaceSpec {
        surface: "node_process_network_control",
        crate_name: "cynic-node",
        owner: "cynic-node",
        runtime_principal: true,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic", "/var/log/cynic"],
        allowed_writers: &["root:root", "cynic:cynic"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/config/",
            "src/usecases/",
            "src/domain/",
            "src/adapters/",
            "src/network_activation_chain.rs",
            "src/network_actuator.rs",
            "src/network_actuator_contract.rs",
            "src/network_actuator_status.rs",
            "src/network_command_surface.rs",
            "src/network_diagnostic_surface.rs",
            "src/network_evidence.rs",
            "src/network_mesh_apply_plan.rs",
            "src/network_mesh_bundle.rs",
            "src/network_mesh_plan.rs",
            "src/network_root_policy.rs",
            "src/network_runtime_status.rs",
            "src/runtime_supervision/",
            "src/watchdog.rs",
            "src/watchdog/",
            "src/system_probe.rs",
        ],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &[
            "service/process control must remain bounded and not rely on checked-out worktree",
            "network plans should be projected via stable runtime contracts",
        ],
    },
    OwnershipSurfaceSpec {
        surface: "coordination_memory_runtime",
        crate_name: "cynic-kernel",
        owner: "cynic-kernel",
        runtime_principal: false,
        expected_owner_filesystem: &["/tmp/cynic/reports", "/var/lib/cynic/reports"],
        allowed_writers: &["cynic:cynic", "root:root"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/domain/",
            "src/storage/",
            "src/api/rest/",
            "src/infra/",
            "src/api/",
        ],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &["kernel should retain coordination memory, not host lifecycle authority"],
    },
    OwnershipSurfaceSpec {
        surface: "inference_runtime",
        crate_name: "cynic-kernel",
        owner: "cynic-kernel",
        runtime_principal: false,
        expected_owner_filesystem: &["/etc/cynic"],
        allowed_writers: &["cynic:cynic", "root:root"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/domain/",
            "src/backends/",
            "src/api/rest/",
            "src/api/rest/observe.rs",
            "src/infra/",
        ],
        must_not_depend_on_project_root: true,
        migration_target: Some("cynic-inference"),
        anti_patterns: &[
            "model and backend ownership should move to cynic-inference",
            "routing and sloting capacity should be owned by inference layer",
        ],
    },
    OwnershipSurfaceSpec {
        surface: "fleet_observation",
        crate_name: "cynic-kernel",
        owner: "cynic-kernel",
        runtime_principal: false,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic"],
        allowed_writers: &["root:root", "cynic:cynic"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/infra/probes/fleet.rs",
            "src/infra/probes/fleet_inventory.rs",
        ],
        must_not_depend_on_project_root: true,
        migration_target: Some("split:cynic-host+cynic-node"),
        anti_patterns: &["fleet projection should be assembled from host/node outputs"],
    },
    OwnershipSurfaceSpec {
        surface: "cli_dev_operator_surfaces",
        crate_name: "cynic-cli",
        owner: "cynic-cli",
        runtime_principal: true,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic", "/usr/local/bin/cynic"],
        allowed_writers: &["root:root", "cynic:cynic"],
        mode: "both",
        dev_only: true,
        scan_paths: &["src/", "tests/"],
        must_not_depend_on_project_root: false,
        migration_target: None,
        anti_patterns: &[
            "installed/runtime split should be explicit in runtime-context outputs",
            "no hidden runtime discovery from CLI-only logic",
        ],
    },
    OwnershipSurfaceSpec {
        surface: "mcp_projection",
        crate_name: "cynic-mcp",
        owner: "cynic-mcp",
        runtime_principal: false,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic"],
        allowed_writers: &["cynic:cynic", "user:user"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/routers/",
            "src/local_tools.rs",
            "src/helpers_split/",
            "src/proxy.rs",
        ],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &["MCP should remain projection only; no core authority ownership"],
    },
    OwnershipSurfaceSpec {
        surface: "protocol_wire_contracts",
        crate_name: "cynic-protocol",
        owner: "cynic-protocol",
        runtime_principal: false,
        expected_owner_filesystem: &[],
        allowed_writers: &["cynic:root", "root:root"],
        mode: "library",
        dev_only: false,
        scan_paths: &["src/"],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &["protocol should stay as typed contract boundaries only"],
    },
    OwnershipSurfaceSpec {
        surface: "inference_discovery",
        crate_name: "cynic-inference",
        owner: "cynic-inference",
        runtime_principal: true,
        expected_owner_filesystem: &["/etc/cynic", "/var/lib/cynic", "/var/cache/cynic"],
        allowed_writers: &["cynic:cynic", "root:root"],
        mode: "runtime_authority",
        dev_only: false,
        scan_paths: &[
            "src/application/",
            "src/adapters/",
            "src/domain/",
            "src/config_loader.rs",
            "src/ods_discovery.rs",
            "src/inference.rs",
            "src/embedding.rs",
        ],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &["LLM/model ownership should stay here"],
    },
    OwnershipSurfaceSpec {
        surface: "askesis_reasoning_tool",
        crate_name: "cynic-askesis",
        owner: "cynic-askesis",
        runtime_principal: false,
        expected_owner_filesystem: &["/var/lib/cynic"],
        allowed_writers: &["user:user", "cynic:cynic"],
        mode: "tool",
        dev_only: false,
        scan_paths: &["src/", "tests/"],
        must_not_depend_on_project_root: true,
        migration_target: None,
        anti_patterns: &["askesis should stay isolated from core runtime authority"],
    },
];

#[derive(Serialize)]
pub(crate) struct OwnershipAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) summary: OwnershipAuditSummary,
    pub(crate) surfaces: Vec<OwnershipSurfaceAudit>,
}

#[derive(Serialize)]
pub(crate) struct OwnershipAuditSummary {
    pub(crate) surface_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) migration_count: usize,
    pub(crate) project_root_dependency_count: usize,
}

#[derive(Serialize)]
pub(crate) struct OwnershipSurfaceAudit {
    pub(crate) surface: &'static str,
    pub(crate) crate_owner: &'static str,
    pub(crate) runtime_principal: bool,
    pub(crate) dev_only: bool,
    pub(crate) expected_owner_filesystem: Vec<&'static str>,
    pub(crate) allowed_writers: Vec<&'static str>,
    pub(crate) runtime_mode: &'static str,
    pub(crate) depends_on_cynic_project_root: bool,
    pub(crate) migration_target: Option<&'static str>,
    pub(crate) status: &'static str,
    pub(crate) project_root_evidence: Vec<String>,
    pub(crate) anti_patterns: Vec<&'static str>,
}

#[derive(Debug)]
struct OwnershipSurfaceSpec {
    surface: &'static str,
    crate_name: &'static str,
    owner: &'static str,
    runtime_principal: bool,
    expected_owner_filesystem: &'static [&'static str],
    allowed_writers: &'static [&'static str],
    mode: &'static str,
    dev_only: bool,
    scan_paths: &'static [&'static str],
    must_not_depend_on_project_root: bool,
    migration_target: Option<&'static str>,
    anti_patterns: &'static [&'static str],
}

pub(crate) fn audit_ownership_surfaces() -> OwnershipAuditReport {
    let mut blocked_count = 0usize;
    let mut migration_count = 0usize;
    let mut root_dep_count = 0usize;

    let surfaces = OWNERSHIP_SURFACES
        .iter()
        .map(|spec| {
            let depends_on_project_root = scans_for_project_root(spec);
            if depends_on_project_root {
                root_dep_count += 1;
            }
            let has_home_owner = owns_user_home(spec);
            if has_home_owner && !spec.dev_only && spec.mode == "runtime_authority" {
                blocked_count += 1;
            }

            let status = if spec.migration_target.is_some() {
                migration_count += 1;
                "migrate"
            } else if has_home_owner && !spec.dev_only && spec.mode == "runtime_authority" {
                "blocked"
            } else if depends_on_project_root && spec.must_not_depend_on_project_root {
                blocked_count += 1;
                "blocked"
            } else {
                "ok"
            };

            OwnershipSurfaceAudit {
                surface: spec.surface,
                crate_owner: spec.owner,
                runtime_principal: spec.runtime_principal,
                dev_only: spec.dev_only,
                expected_owner_filesystem: spec.expected_owner_filesystem.to_vec(),
                allowed_writers: spec.allowed_writers.to_vec(),
                runtime_mode: spec.mode,
                depends_on_cynic_project_root: depends_on_project_root,
                migration_target: spec.migration_target,
                status,
                project_root_evidence: find_project_root_evidence(spec),
                anti_patterns: spec.anti_patterns.to_vec(),
            }
        })
        .collect::<Vec<_>>();

    let status = if blocked_count == 0 { "ok" } else { "blocked" };
    OwnershipAuditReport {
        schema: "cynic.cli.ownership_audit.v1",
        status,
        summary: OwnershipAuditSummary {
            surface_count: OWNERSHIP_SURFACES.len(),
            blocked_count,
            migration_count,
            project_root_dependency_count: root_dep_count,
        },
        surfaces,
    }
}

fn owns_user_home(spec: &OwnershipSurfaceSpec) -> bool {
    spec.expected_owner_filesystem
        .iter()
        .any(|path| path.starts_with("/home/"))
}

fn scans_for_project_root(spec: &OwnershipSurfaceSpec) -> bool {
    !find_project_root_evidence(spec).is_empty()
}

fn find_project_root_evidence(spec: &OwnershipSurfaceSpec) -> Vec<String> {
    scan_files_for_project_root_dependency(
        crate_root(spec.crate_name).as_deref(),
        spec.scan_paths,
        PROJECT_ROOT_TOKEN,
    )
}

fn scan_files_for_project_root_dependency(
    crate_root: Option<&Path>,
    scan_paths: &[&str],
    token: &str,
) -> Vec<String> {
    let Some(crate_root) = crate_root else {
        return Vec::new();
    };

    let mut matches = BTreeSet::new();
    for file in rust_files_under(crate_root) {
        if !matches_scan_scope(&file, crate_root, scan_paths) {
            continue;
        }

        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        if file
            .file_name()
            .is_some_and(|name| name == "ownership_audit.rs")
        {
            continue;
        }

        if contains_project_root_dependency(&content, token) {
            matches.insert(relative_path(crate_root, &file));
        }
    }

    matches.into_iter().collect()
}

fn contains_project_root_dependency(content: &str, token: &str) -> bool {
    if !content.contains(token) {
        return false;
    }

    let lookup_patterns = [
        format!("env::var(\"{token}\""),
        format!("env::var('{token}'"),
        format!("env::var_os(\"{token}\""),
        format!("env::var_os('{token}'"),
        format!("std::env::var(\"{token}\""),
        format!("std::env::var('{token}'"),
        format!("std::env::var_os(\"{token}\""),
        format!("std::env::var_os('{token}'"),
    ];

    lookup_patterns
        .iter()
        .any(|pattern| content.contains(pattern))
}

fn matches_scan_scope(file: &Path, crate_root: &Path, scan_paths: &[&str]) -> bool {
    let relative = relative_path(crate_root, file);
    scan_paths.is_empty()
        || scan_paths
            .iter()
            .map(|path| path.trim_end_matches('/'))
            .any(|path| {
                let with_slash = path.to_string() + "/";
                relative == path || relative.starts_with(with_slash.as_str())
            })
}

fn crate_root(crate_name: &str) -> Option<PathBuf> {
    let root = workspace_root().join("crates").join(crate_name);
    if root.exists() { Some(root) } else { None }
}

#[cfg(test)]
mod tests {
    use super::audit_ownership_surfaces;

    #[test]
    fn ownership_audit_reports_expected_schema() {
        let report = audit_ownership_surfaces();
        assert_eq!(report.schema, "cynic.cli.ownership_audit.v1");
        assert!(report.summary.surface_count >= 8);
    }

    #[test]
    fn ownership_audit_flags_migration_targets() {
        let report = audit_ownership_surfaces();
        assert!(
            report
                .surfaces
                .iter()
                .any(|entry| entry.migration_target.is_some() && !entry.status.is_empty())
        );
    }

    #[test]
    fn ownership_audit_rejects_non_dev_runtime_authority_under_home() {
        let report = audit_ownership_surfaces();
        for surface in report.surfaces {
            if surface.runtime_mode == "runtime_authority" && !surface.dev_only {
                assert!(
                    !surface
                        .expected_owner_filesystem
                        .iter()
                        .any(|path| path.starts_with("/home/")),
                    "{} should not own runtime authority under /home",
                    surface.surface
                );
            }
        }
    }
}
