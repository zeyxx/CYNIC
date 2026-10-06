//! `cynic-host-exec pve-helper-bundle` — stage the generated sudoers membrane
//! and a machine-readable manifest for HumanAdmin bootstrap on the Proxmox host.
//!
//! Thin glue over `crate::privilege::pve_helper`: the install-site policy says
//! WHERE artifacts land, the capability catalog says WHAT is granted; this
//! surface only stages, projects, and reports.
//!
//! NO EXECUTOR IS STAGED. The bounded executor is the `cynic` binary itself
//! (`privilege exec`), installed by the normal binary install path. The shell
//! script this surface used to stage existed only because there was no single
//! binary to name; it is gone.

use std::path::Path;

use serde::Serialize;

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
    sudoers_source: String,
    sudoers_path: String,
    helper_path: String,
    catalog_path: String,
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
        sudoers_source: String::new(),
        sudoers_path: String::new(),
        helper_path: String::new(),
        catalog_path: String::new(),
        capability_ids: Vec::new(),
        manifest: None,
        blockers: Vec::new(),
    };

    // Policy (where) + catalog (what). A missing catalog is an error: an empty
    // membrane would look like a valid install.
    let (policy, catalog) = match PveHelperPolicy::load_with_catalog(&policy_path) {
        Ok(pair) => pair,
        Err(error) => {
            report
                .blockers
                .push(format!("policy_or_catalog_unreadable: {}", error.0));
            return finish(report);
        }
    };
    report.sudoers_path = policy.sudoers_path.clone();
    report.helper_path = policy.helper_path.clone();
    report.catalog_path = policy.catalog_path.clone();
    report.capability_ids = catalog
        .capabilities
        .iter()
        .filter(|capability| capability.requires_root)
        .map(|capability| capability.id.clone())
        .collect();

    let projection = policy.sudoers_projection(&catalog);
    if projection.is_empty() {
        report.blockers.push(
            "empty_membrane: the catalog declares no requires_root capability, refusing to stage"
                .to_string(),
        );
        return finish(report);
    }

    if let Err(error) = std::fs::create_dir_all(&output_dir) {
        report
            .blockers
            .push(format!("stage_dir_create_failed: {error}"));
        return finish(report);
    }

    let staged_sudoers = Path::new(&output_dir).join("pve-pct.sudoers");
    report.sudoers_source = staged_sudoers.display().to_string();
    if let Err(error) = std::fs::write(&staged_sudoers, &projection) {
        report
            .blockers
            .push(format!("sudoers_stage_failed: {error}"));
        return finish(report);
    }

    let manifest = pve_helper_bundle_manifest(&policy, &catalog, &staged_sudoers);
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
    for artifact in [staged_sudoers.as_path(), manifest_path.as_path()] {
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

fn finish(report: PveHelperBundleReport) -> Result<std::process::ExitCode, String> {
    let failed = report.status != "staged";
    emit_json(&report, failed)
}
