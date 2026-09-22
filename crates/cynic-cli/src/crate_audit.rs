use std::path::Path;

use serde::Serialize;

use crate::rust_source::{
    line_count, relative_to_workspace, rust_files_under, workspace_crate_names, workspace_root,
};

/// Entrypoint bounds policy: uniform for every workspace member. Which crates
/// exist is observed from the workspace — never a hardcoded table.
const DEFAULT_MAIN_THRESHOLD: usize = 300;
const DEFAULT_LIB_THRESHOLD: usize = 220;

const HOTSPOT_LINE_THRESHOLD: usize = 400;

#[derive(Serialize)]
pub(crate) struct CrateAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) crate_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) crates: Vec<CrateAuditEntry>,
}

#[derive(Serialize)]
pub(crate) struct CrateAuditEntry {
    pub(crate) crate_name: String,
    pub(crate) status: &'static str,
    pub(crate) crate_root: String,
    pub(crate) rust_file_count: usize,
    pub(crate) has_architecture_boundary_test: bool,
    pub(crate) entrypoints: Vec<EntrypointAudit>,
    pub(crate) hotspots: Vec<FileHotspot>,
    pub(crate) checks: Vec<CrateAuditCheck>,
}

#[derive(Serialize)]
pub(crate) struct EntrypointAudit {
    pub(crate) file_path: String,
    pub(crate) line_count: usize,
    pub(crate) status: &'static str,
    pub(crate) threshold: usize,
}

#[derive(Serialize)]
pub(crate) struct FileHotspot {
    pub(crate) file_path: String,
    pub(crate) line_count: usize,
    pub(crate) status: &'static str,
}

#[derive(Serialize)]
pub(crate) struct CrateAuditCheck {
    pub(crate) name: &'static str,
    pub(crate) status: &'static str,
    pub(crate) detail: String,
}

pub(crate) fn audit_core_crates() -> CrateAuditReport {
    let crates = workspace_crate_names()
        .iter()
        .map(|name| audit_crate(name, DEFAULT_MAIN_THRESHOLD, DEFAULT_LIB_THRESHOLD))
        .collect::<Vec<_>>();
    let ok_count = crates.iter().filter(|entry| entry.status == "ok").count();
    let blocked_count = crates.len().saturating_sub(ok_count);
    CrateAuditReport {
        schema: "cynic.cli.crate_audit.v1",
        status: if blocked_count == 0 { "ok" } else { "blocked" },
        rule: "core crates must keep executable boundary tests, bounded entrypoints, and explicit visibility into source hotspots",
        crate_count: crates.len(),
        ok_count,
        blocked_count,
        crates,
    }
}

fn audit_crate(crate_name: &str, main_threshold: usize, lib_threshold: usize) -> CrateAuditEntry {
    let crate_root = workspace_root().join("crates").join(crate_name);
    let rust_files = rust_files_under(&crate_root);
    let has_architecture_boundary_test = crate_root.join("tests/architecture_boundary.rs").exists();
    let entrypoints = vec![
        audit_entrypoint(&crate_root.join("src/main.rs"), main_threshold),
        audit_entrypoint(&crate_root.join("src/lib.rs"), lib_threshold),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let hotspots = rust_files
        .iter()
        .filter_map(|path| {
            let line_count = line_count(path)?;
            (line_count >= HOTSPOT_LINE_THRESHOLD).then(|| FileHotspot {
                file_path: relative_to_workspace(path),
                line_count,
                status: "observe",
            })
        })
        .collect::<Vec<_>>();
    let checks = vec![
        CrateAuditCheck {
            name: "architecture_boundary_test_present",
            status: if has_architecture_boundary_test {
                "ok"
            } else {
                "blocked"
            },
            detail: crate_root
                .join("tests/architecture_boundary.rs")
                .display()
                .to_string(),
        },
        CrateAuditCheck {
            name: "rust_files_present",
            status: if rust_files.is_empty() {
                "blocked"
            } else {
                "ok"
            },
            detail: format!("rust_file_count={}", rust_files.len()),
        },
        CrateAuditCheck {
            name: "entrypoints_bounded",
            status: if entrypoints.iter().all(|entry| entry.status == "ok") {
                "ok"
            } else {
                "blocked"
            },
            detail: if entrypoints.is_empty() {
                "no main.rs/lib.rs found".to_string()
            } else {
                format!("entrypoint_count={}", entrypoints.len())
            },
        },
        CrateAuditCheck {
            name: "hotspots_observed",
            status: "ok",
            detail: format!("hotspot_count={}", hotspots.len()),
        },
    ];
    let status = if checks.iter().all(|check| check.status == "ok") {
        "ok"
    } else {
        "blocked"
    };
    CrateAuditEntry {
        crate_name: crate_name.to_string(),
        status,
        crate_root: crate_root.display().to_string(),
        rust_file_count: rust_files.len(),
        has_architecture_boundary_test,
        entrypoints,
        hotspots,
        checks,
    }
}

fn audit_entrypoint(path: &Path, threshold: usize) -> Option<EntrypointAudit> {
    path.exists().then(|| {
        let line_count = line_count(path).unwrap_or_default();
        EntrypointAudit {
            file_path: relative_to_workspace(path),
            line_count,
            status: if line_count <= threshold {
                "ok"
            } else {
                "blocked"
            },
            threshold,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::audit_core_crates;

    #[test]
    fn crate_audit_reports_core_crates() {
        let report = audit_core_crates();
        assert_eq!(report.schema, "cynic.cli.crate_audit.v1");
        assert!(report.crate_count >= 6);
        assert!(
            report
                .crates
                .iter()
                .any(|entry| entry.crate_name == "cynic-cli" && entry.rust_file_count > 0)
        );
        assert!(
            report
                .crates
                .iter()
                .any(|entry| entry.crate_name == "cynic-node" && !entry.entrypoints.is_empty())
        );
    }
}
