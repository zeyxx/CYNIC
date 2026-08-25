//! `cynic-host-exec pve-helper-bundle` — stage the root-owned bounded
//! executor, the generated sudoers membrane, and a machine-readable manifest
//! for HumanAdmin bootstrap on the Proxmox host.
//!
//! Thin glue over `crate::privilege::pve_helper`: policy is DATA
//! (infra/privilege/pve-pct-helper.policy.json); this surface only stages,
//! projects, and reports.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::bounded_helper::BoundedHelperProjection;
use crate::host_exec_cli::args;
use crate::host_exec_cli::io::emit_json;
use crate::privilege::{
    DEFAULT_POLICY_PATH, PveHelperBundleManifest, PveHelperPolicy, pve_helper_bundle_manifest,
};

const PVE_HELPER_BUNDLE_REPORT_SCHEMA: &str = "cynic.host.pve_helper_bundle_report.v1";

#[derive(Debug, Serialize)]
struct PveHelperBundleReport {
    schema: &'static str,
    status: String,
    staged_dir: String,
    helper_source: String,
    sudoers_path: String,
    helper_path: String,
    capability_ids: Vec<String>,
    manifest: Option<PveHelperBundleManifest>,
    blockers: Vec<String>,
}

pub(crate) fn execute_pve_helper_bundle(
    args_slice: &[String],
) -> Result<std::process::ExitCode, String> {
    let output_dir = args::flag_value(args_slice, "--output-dir")
        .ok_or_else(|| "pve-helper-bundle requires --output-dir <path>".to_string())?;
    let policy_path =
        args::flag_value(args_slice, "--policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());

    let mut report = PveHelperBundleReport {
        schema: PVE_HELPER_BUNDLE_REPORT_SCHEMA,
        status: "blocked".to_string(),
        staged_dir: output_dir.clone(),
        helper_source: String::new(),
        sudoers_path: String::new(),
        helper_path: String::new(),
        capability_ids: Vec::new(),
        manifest: None,
        blockers: Vec::new(),
    };

    let Ok(policy) = PveHelperPolicy::load(&policy_path) else {
        report
            .blockers
            .push(format!("policy_unreadable_or_invalid: {policy_path}"));
        return finish(report);
    };
    report.sudoers_path = policy.sudoers_path.clone();
    report.helper_path = policy.helper_path.clone();
    report.capability_ids = policy
        .capabilities
        .iter()
        .map(|capability| capability.id.clone())
        .collect();

    let executor_source = find_executor_source(&policy);
    let Some(executor_source) = executor_source else {
        report.blockers.push(
            "executor_source_missing: expected infra/host-bootstrap/pve/pct-bounded-exec.sh \
             beside the repo root"
                .to_string(),
        );
        return finish(report);
    };
    report.helper_source = executor_source.display().to_string();

    // Stage: copy executor (0755) + write generated sudoers projection.
    if let Err(error) = std::fs::create_dir_all(&output_dir) {
        report
            .blockers
            .push(format!("stage_dir_create_failed: {error}"));
        return finish(report);
    }
    let staged_executor = Path::new(&output_dir).join("pct-bounded-exec");
    if let Err(error) = std::fs::copy(&executor_source, &staged_executor) {
        report
            .blockers
            .push(format!("executor_stage_failed: {error}"));
        return finish(report);
    }
    use std::os::unix::fs::PermissionsExt;
    if let Err(error) =
        std::fs::set_permissions(&staged_executor, std::fs::Permissions::from_mode(0o755))
    {
        report
            .blockers
            .push(format!("executor_chmod_failed: {error}"));
        return finish(report);
    }
    let staged_sudoers = Path::new(&output_dir).join("pve-pct.sudoers");
    let projection: BoundedHelperProjection = policy.projection();
    if let Err(error) = std::fs::write(&staged_sudoers, projection.sudoers_projection()) {
        report
            .blockers
            .push(format!("sudoers_stage_failed: {error}"));
        return finish(report);
    }

    let manifest = pve_helper_bundle_manifest(&policy, &staged_executor);
    let manifest_path = Path::new(&output_dir).join("manifest.json");
    match serde_json::to_string_pretty(&manifest) {
        Ok(json) => {
            if let Err(error) = std::fs::write(&manifest_path, format!("{json}\n")) {
                report
                    .blockers
                    .push(format!("manifest_write_failed: {error}"));
                return finish(report);
            }
        }
        Err(error) => {
            report
                .blockers
                .push(format!("manifest_serialize_failed: {error}"));
            return finish(report);
        }
    }
    report.manifest = Some(manifest);

    // Read-back proof: every expected artifact exists and is non-empty.
    for artifact in [
        staged_executor.as_path(),
        staged_sudoers.as_path(),
        manifest_path.as_path(),
    ] {
        match std::fs::metadata(artifact) {
            Ok(metadata) if metadata.len() > 0 => {}
            Ok(_) => report
                .blockers
                .push(format!("empty_artifact: {}", artifact.display())),
            Err(error) => report
                .blockers
                .push(format!("missing_artifact {}: {error}", artifact.display())),
        }
    }
    if report.blockers.is_empty() {
        report.status = "staged".to_string();
    }
    finish(report)
}

fn find_executor_source(policy: &PveHelperPolicy) -> Option<PathBuf> {
    policy
        .executor_source_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn finish(report: PveHelperBundleReport) -> Result<std::process::ExitCode, String> {
    let failed = report.status != "staged";
    emit_json(&report, failed)
}
