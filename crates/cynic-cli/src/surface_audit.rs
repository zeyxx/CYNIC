use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::surface_registry::{CanonicalCliSurface, canonical_cli_surfaces};

#[derive(Serialize)]
pub(crate) struct SurfaceAuditReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) rule: &'static str,
    pub(crate) surface_count: usize,
    pub(crate) ok_count: usize,
    pub(crate) blocked_count: usize,
    pub(crate) surfaces: Vec<SurfaceAuditEntry>,
}

#[derive(Serialize)]
pub(crate) struct SurfaceAuditEntry {
    pub(crate) id: &'static str,
    pub(crate) status: &'static str,
    pub(crate) owner_crate: &'static str,
    pub(crate) domain_owner: &'static str,
    pub(crate) adapter_module: &'static str,
    pub(crate) adapter_path: String,
    pub(crate) owner_module_paths: Vec<String>,
    pub(crate) mutation_policy: &'static str,
    pub(crate) checks: Vec<SurfaceAuditCheck>,
}

#[derive(Serialize)]
pub(crate) struct SurfaceAuditCheck {
    pub(crate) name: &'static str,
    pub(crate) status: &'static str,
    pub(crate) detail: String,
}

pub(crate) fn audit_canonical_surfaces() -> SurfaceAuditReport {
    let surfaces = canonical_cli_surfaces();
    let ids = duplicate_ids(&surfaces);
    let entries = surfaces
        .iter()
        .map(|surface| audit_surface(surface, &ids))
        .collect::<Vec<_>>();
    let ok_count = entries.iter().filter(|entry| entry.status == "ok").count();
    let blocked_count = entries.len().saturating_sub(ok_count);
    SurfaceAuditReport {
        schema: "cynic.cli.surface_audit.v1",
        status: if blocked_count == 0 { "ok" } else { "blocked" },
        rule: "canonical CLI surfaces must be backed by real adapter modules, declared owners, mutation policies, evidence, and anti-responsibilities",
        surface_count: entries.len(),
        ok_count,
        blocked_count,
        surfaces: entries,
    }
}

fn audit_surface(
    surface: &CanonicalCliSurface,
    duplicate_ids: &BTreeSet<&'static str>,
) -> SurfaceAuditEntry {
    let adapter_path = adapter_module_path(surface.adapter_module);
    let owner_module_paths = surface
        .owner_modules
        .iter()
        .map(|module| owner_module_path(surface.owner_crate, module))
        .collect::<Vec<_>>();
    let adapter_source = read_adapter_source(&adapter_path).ok();
    let mut checks = vec![
        check_unique_id(surface, duplicate_ids),
        check_known_owner(surface),
        check_known_mutation_policy(surface),
        check_non_empty("layer_declared", surface.layer),
        check_non_empty("domain_owner_declared", surface.domain_owner),
        check_non_empty("responsibility_declared", surface.responsibility),
        check_vec_non_empty("owner_modules_declared", &surface.owner_modules),
        check_vec_non_empty("owner_call_markers_declared", &surface.owner_call_markers),
        check_adapter_exists(&adapter_path),
        check_owner_calls_present(surface, adapter_source.as_deref(), &adapter_path),
        check_mutation_policy_matches_source(surface, adapter_source.as_deref(), &adapter_path),
        check_vec_non_empty(
            "canonical_entrypoints_declared",
            &surface.canonical_entrypoints,
        ),
        check_vec_non_empty("evidence_declared", &surface.evidence),
        check_vec_non_empty(
            "anti_responsibilities_declared",
            &surface.anti_responsibilities,
        ),
    ];
    checks.extend(
        owner_module_paths
            .iter()
            .map(|path| check_owner_module_exists(path)),
    );
    let status = if checks.iter().all(|check| check.status == "ok") {
        "ok"
    } else {
        "blocked"
    };
    SurfaceAuditEntry {
        id: surface.id,
        status,
        owner_crate: surface.owner_crate,
        domain_owner: surface.domain_owner,
        adapter_module: surface.adapter_module,
        adapter_path: adapter_path.display().to_string(),
        owner_module_paths: owner_module_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        mutation_policy: surface.mutation_policy,
        checks,
    }
}

fn duplicate_ids(surfaces: &[CanonicalCliSurface]) -> BTreeSet<&'static str> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for surface in surfaces {
        if !seen.insert(surface.id) {
            duplicates.insert(surface.id);
        }
    }
    duplicates
}

fn check_unique_id(
    surface: &CanonicalCliSurface,
    duplicate_ids: &BTreeSet<&'static str>,
) -> SurfaceAuditCheck {
    let duplicate = duplicate_ids.contains(surface.id);
    SurfaceAuditCheck {
        name: "unique_id",
        status: if duplicate { "blocked" } else { "ok" },
        detail: if duplicate {
            format!("duplicate id {}", surface.id)
        } else {
            format!("id {} is unique", surface.id)
        },
    }
}

fn check_known_owner(surface: &CanonicalCliSurface) -> SurfaceAuditCheck {
    let known = matches!(
        surface.owner_crate,
        "cynic-cli" | "cynic-host" | "cynic-node" | "cynic-protocol"
    );
    SurfaceAuditCheck {
        name: "known_owner_crate",
        status: if known { "ok" } else { "blocked" },
        detail: format!("owner_crate={}", surface.owner_crate),
    }
}

fn check_known_mutation_policy(surface: &CanonicalCliSurface) -> SurfaceAuditCheck {
    let known = matches!(
        surface.mutation_policy,
        "read_only"
            | "read_only_temp_artifacts"
            | "artifact_generation"
            | "explicit_apply_only"
            | "read_only_by_default_explicit_apply_for_actuators"
    );
    SurfaceAuditCheck {
        name: "known_mutation_policy",
        status: if known { "ok" } else { "blocked" },
        detail: format!("mutation_policy={}", surface.mutation_policy),
    }
}

fn check_non_empty(name: &'static str, value: &str) -> SurfaceAuditCheck {
    SurfaceAuditCheck {
        name,
        status: if value.trim().is_empty() {
            "blocked"
        } else {
            "ok"
        },
        detail: if value.trim().is_empty() {
            "empty".to_string()
        } else {
            value.to_string()
        },
    }
}

fn check_vec_non_empty(name: &'static str, values: &[&str]) -> SurfaceAuditCheck {
    SurfaceAuditCheck {
        name,
        status: if values.is_empty() { "blocked" } else { "ok" },
        detail: format!("count={}", values.len()),
    }
}

fn check_adapter_exists(path: &Path) -> SurfaceAuditCheck {
    SurfaceAuditCheck {
        name: "adapter_module_exists",
        status: if path.exists() { "ok" } else { "blocked" },
        detail: path.display().to_string(),
    }
}

fn check_owner_module_exists(path: &Path) -> SurfaceAuditCheck {
    SurfaceAuditCheck {
        name: "owner_module_exists",
        status: if path.exists() { "ok" } else { "blocked" },
        detail: path.display().to_string(),
    }
}

fn check_mutation_policy_matches_source(
    surface: &CanonicalCliSurface,
    source: Option<&str>,
    path: &Path,
) -> SurfaceAuditCheck {
    let Some(source) = source else {
        return SurfaceAuditCheck {
            name: "mutation_policy_matches_source",
            status: "blocked",
            detail: format!("cannot read adapter source {}", path.display()),
        };
    };
    let signals = mutation_signals(&source, surface);
    let allowed = match surface.mutation_policy {
        "read_only" => signals
            .iter()
            .all(|signal| matches!(signal.as_str(), "process_execution")),
        "read_only_temp_artifacts" => signals.iter().all(|signal| {
            matches!(
                signal.as_str(),
                "process_execution" | "filesystem_temp_artifact"
            )
        }),
        "artifact_generation" => signals.iter().all(|signal| {
            matches!(
                signal.as_str(),
                "artifact_generation" | "filesystem_temp_artifact"
            )
        }),
        "explicit_apply_only" => {
            let has_apply_guard = source.contains("requires --apply")
                || source.contains("--apply")
                || surface
                    .canonical_entrypoints
                    .iter()
                    .any(|entrypoint| entrypoint.contains("--apply"));
            has_apply_guard
                && signals.iter().all(|signal| {
                    matches!(
                        signal.as_str(),
                        "explicit_apply" | "artifact_generation" | "filesystem_temp_artifact"
                    )
                })
        }
        "read_only_by_default_explicit_apply_for_actuators" => {
            let has_apply_guard = surface
                .canonical_entrypoints
                .iter()
                .any(|entrypoint| entrypoint.contains("--apply"));
            has_apply_guard
                && signals.iter().all(|signal| {
                    matches!(
                        signal.as_str(),
                        "explicit_apply" | "artifact_generation" | "process_execution"
                    )
                })
        }
        _ => false,
    };
    SurfaceAuditCheck {
        name: "mutation_policy_matches_source",
        status: if allowed { "ok" } else { "blocked" },
        detail: if signals.is_empty() {
            "signals=[]".to_string()
        } else {
            format!("signals={signals:?}")
        },
    }
}

fn check_owner_calls_present(
    surface: &CanonicalCliSurface,
    source: Option<&str>,
    path: &Path,
) -> SurfaceAuditCheck {
    let Some(source) = source else {
        return SurfaceAuditCheck {
            name: "owner_calls_present",
            status: "blocked",
            detail: format!("cannot read adapter source {}", path.display()),
        };
    };
    let missing = surface
        .owner_call_markers
        .iter()
        .copied()
        .filter(|marker| !source.contains(marker))
        .collect::<Vec<_>>();
    SurfaceAuditCheck {
        name: "owner_calls_present",
        status: if missing.is_empty() { "ok" } else { "blocked" },
        detail: if missing.is_empty() {
            format!("markers={}", surface.owner_call_markers.len())
        } else {
            format!("missing={missing:?}")
        },
    }
}

fn mutation_signals(source: &str, surface: &CanonicalCliSurface) -> Vec<String> {
    let mut signals = BTreeSet::new();
    if source.contains("execute_host_apply")
        || source.contains("principal_prepare_apply")
        || source.contains("bridge_install_report")
        || source.contains("ApplyProfile")
        || source.contains("NetworkActuatorCommand::Apply")
        || source.contains(" apply,")
        || source.contains("apply:")
        || source.contains("--apply")
    {
        signals.insert("explicit_apply".to_string());
    }
    if source.contains("write_bridge_bundle")
        || source.contains("write_framework_bootstrap_bundle")
        || source.contains("write_lab_bundle")
        || source.contains("write_guest_bootstrap_bundle")
        || source.contains("output_dir")
        || source.contains("bundle_dir")
    {
        signals.insert("artifact_generation".to_string());
    }
    if source.contains("std::fs::write")
        || source.contains("std::fs::create_dir_all")
        || source.contains("mktemp")
    {
        signals.insert("filesystem_temp_artifact".to_string());
    }
    if source.contains("ProcessCommand::new")
        || source.contains("std::process::Command::new")
        || source.contains("Command::new")
    {
        signals.insert("process_execution".to_string());
    }

    // Framework bootstrap writes a user-selected artifact directory; it is not an OS apply.
    if surface.mutation_policy == "artifact_generation" {
        signals.remove("explicit_apply");
    }

    signals.into_iter().collect()
}

fn read_adapter_source(path: &Path) -> Result<String, std::io::Error> {
    let mut source = std::fs::read_to_string(path)?;
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(source);
    };
    let Some(parent) = path.parent() else {
        return Ok(source);
    };
    let module_dir = parent.join(stem);
    if module_dir.is_dir() {
        for child in rust_files_under(&module_dir)? {
            source.push('\n');
            source.push_str(&std::fs::read_to_string(child)?);
        }
    }
    Ok(source)
}

fn rust_files_under(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn adapter_module_path(adapter_module: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let parts = adapter_module.split("::").collect::<Vec<_>>();
    match parts.as_slice() {
        [module] => path.push(format!("{module}.rs")),
        [module, child] => {
            let module_file = path.join(format!("{module}.rs"));
            let nested_file = path.join(module).join(format!("{child}.rs"));
            if nested_file.exists() || !module_file.exists() {
                path = nested_file;
            } else {
                path = module_file;
            }
        }
        _ => {
            for part in &parts[..parts.len().saturating_sub(1)] {
                path.push(part);
            }
            if let Some(last) = parts.last() {
                path.push(format!("{last}.rs"));
            }
        }
    }
    path
}

fn owner_module_path(owner_crate: &str, module: &str) -> PathBuf {
    if let Some((crate_name, module_name)) = module.split_once("::")
        && matches!(
            crate_name,
            "cynic-host" | "cynic-node" | "cynic-protocol" | "cynic-cli"
        )
    {
        return crate_module_path(crate_name, module_name);
    }
    crate_module_path(owner_crate, module)
}

fn crate_module_path(crate_name: &str, module: &str) -> PathBuf {
    let crate_dir = match crate_name {
        "cynic-cli" => env!("CARGO_MANIFEST_DIR").to_string(),
        "cynic-host" => format!("{}/../cynic-host", env!("CARGO_MANIFEST_DIR")),
        "cynic-node" => format!("{}/../cynic-node", env!("CARGO_MANIFEST_DIR")),
        "cynic-protocol" => format!("{}/../cynic-protocol", env!("CARGO_MANIFEST_DIR")),
        _ => env!("CARGO_MANIFEST_DIR").to_string(),
    };
    let mut path = PathBuf::from(crate_dir).join("src");
    let parts = module.split("::").collect::<Vec<_>>();
    match parts.as_slice() {
        [single] => {
            let file = path.join(format!("{single}.rs"));
            let mod_file = path.join(single).join("mod.rs");
            if mod_file.exists() { mod_file } else { file }
        }
        _ => {
            for part in &parts[..parts.len().saturating_sub(1)] {
                path.push(part);
            }
            if let Some(last) = parts.last() {
                let file = path.join(format!("{last}.rs"));
                let mod_file = path.join(last).join("mod.rs");
                if mod_file.exists() {
                    return mod_file;
                }
                return file;
            }
            path
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_surface_audit_is_ok() {
        let report = audit_canonical_surfaces();
        assert_eq!(report.status, "ok");
        assert_eq!(report.blocked_count, 0);
    }
}
