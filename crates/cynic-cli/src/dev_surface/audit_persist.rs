use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::fs_json::write_pretty_json_file;
use crate::kernel_boundary_audit::KernelAuditReport;

const DEFAULT_REPORT_DIR: &str = "/var/lib/cynic/reports";
const FALLBACK_REPORT_DIR: &str = "/tmp/cynic/reports";

pub(crate) fn maybe_persist_kernel_audit_report(
    report: &KernelAuditReport,
    output: Option<PathBuf>,
    persist: bool,
) -> Result<(), String> {
    maybe_persist_dev_audit_report(report, output, persist, "kernel-audit.json")
}

pub(crate) fn maybe_persist_dev_audit_report<T: Serialize>(
    report: &T,
    output: Option<PathBuf>,
    persist: bool,
    filename: &'static str,
) -> Result<(), String> {
    let report_dir_env = std::env::var("CYNIC_REPORT_DIR").ok();
    let is_default_dir = report_dir_env.is_none();
    let fallback_dir = Path::new(FALLBACK_REPORT_DIR);
    let default_dir = Path::new(report_dir_env.as_deref().unwrap_or(DEFAULT_REPORT_DIR));
    let explicit_output = output.is_some();
    let Some(path) = output.or_else(|| persist.then(|| default_dir.join(filename))) else {
        return Ok(());
    };

    if let Err(error) = write_dev_audit_report(&path, report) {
        if explicit_output || !is_default_dir {
            return Err(error);
        }

        let fallback = fallback_dir.join(filename);
        if let Err(fallback_error) = write_dev_audit_report(&fallback, report) {
            return Err(format!(
                "{error}; fallback {} failed: {fallback_error}",
                fallback.display()
            ));
        }

        eprintln!(
            "warning: default audit report path {} is not writable, falling back to {}",
            path.display(),
            fallback.display()
        );
    }

    Ok(())
}

fn write_dev_audit_report<T: Serialize>(path: &Path, report: &T) -> Result<(), String> {
    write_pretty_json_file(path, report, "audit report")
}
