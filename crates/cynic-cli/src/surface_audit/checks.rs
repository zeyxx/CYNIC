use std::collections::BTreeSet;
use std::path::Path;

use crate::surface_audit::paths::{adapter_module_path, owner_module_path, read_adapter_source};
use crate::surface_audit::types::{SurfaceAuditCheck, SurfaceAuditEntry};
use crate::surface_registry::CanonicalCliSurface;

pub(crate) fn duplicate_ids(surfaces: &[CanonicalCliSurface]) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for surface in surfaces {
        if !seen.insert(surface.id.clone()) {
            duplicates.insert(surface.id.clone());
        }
    }
    duplicates
}

pub(crate) fn audit_surface(
    surface: &CanonicalCliSurface,
    duplicate_ids: &BTreeSet<String>,
) -> SurfaceAuditEntry {
    let adapter_path = adapter_module_path(&surface.owner_crate, &surface.adapter_module);
    let owner_module_paths = surface
        .owner_modules
        .iter()
        .map(|module| owner_module_path(&surface.owner_crate, module))
        .collect::<Vec<_>>();
    let adapter_source = read_adapter_source(&adapter_path).ok();
    let mut checks = vec![
        check_unique_id(surface, duplicate_ids),
        check_known_owner(surface),
        check_known_mutation_policy(surface),
        check_non_empty("layer_declared", &surface.layer),
        check_non_empty("domain_owner_declared", &surface.domain_owner),
        check_non_empty("responsibility_declared", &surface.responsibility),
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
        id: surface.id.clone(),
        status,
        owner_crate: surface.owner_crate.clone(),
        domain_owner: surface.domain_owner.clone(),
        adapter_module: surface.adapter_module.clone(),
        adapter_path: adapter_path.display().to_string(),
        owner_module_paths: owner_module_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        mutation_policy: surface.mutation_policy.clone(),
        checks,
    }
}

fn check_unique_id(
    surface: &CanonicalCliSurface,
    duplicate_ids: &BTreeSet<String>,
) -> SurfaceAuditCheck {
    let duplicate = duplicate_ids.contains(&surface.id);
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
        surface.owner_crate.as_str(),
        "cynic-cli" | "cynic-host" | "cynic-node" | "cynic-protocol" | "cynic-inference"
    );
    SurfaceAuditCheck {
        name: "known_owner_crate",
        status: if known { "ok" } else { "blocked" },
        detail: format!("owner_crate={}", surface.owner_crate),
    }
}

fn check_known_mutation_policy(surface: &CanonicalCliSurface) -> SurfaceAuditCheck {
    let known = matches!(
        surface.mutation_policy.as_str(),
        "read_only"
            | "read_only_temp_artifacts"
            | "artifact_generation"
            | "explicit_apply_only"
            | "explicit_apply_only_after_signature_digest_epoch_node_and_slot_preflight"
            | "plan_only_no_transfer_no_checkout_no_grant_emission"
            | "read_only_by_default_explicit_apply_bounded_restart"
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

fn check_vec_non_empty(name: &'static str, values: &[String]) -> SurfaceAuditCheck {
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
    let signals = mutation_signals(source, surface);
    let allowed = match surface.mutation_policy.as_str() {
        "read_only" | "plan_only_no_transfer_no_checkout_no_grant_emission" => signals
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
        "explicit_apply_only"
        | "explicit_apply_only_after_signature_digest_epoch_node_and_slot_preflight" => {
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
        "read_only_by_default_explicit_apply_for_actuators"
        | "read_only_by_default_explicit_apply_bounded_restart" => {
            let has_apply_guard = surface
                .canonical_entrypoints
                .iter()
                .any(|entrypoint| entrypoint.contains("--apply"));
            has_apply_guard
                && signals.iter().all(|signal| {
                    matches!(
                        signal.as_str(),
                        "explicit_apply"
                            | "artifact_generation"
                            | "filesystem_temp_artifact"
                            | "process_execution"
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
        .filter(|marker| !source.contains(marker.as_str()))
        .cloned()
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

    if surface.mutation_policy == "artifact_generation" {
        signals.remove("explicit_apply");
    }

    signals.into_iter().collect()
}
