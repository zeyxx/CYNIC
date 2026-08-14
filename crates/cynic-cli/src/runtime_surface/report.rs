use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::fs_json::write_pretty_json_file;
use crate::{emit_json, fail};

mod build;
mod catalog;
mod summary;
mod types;

use build::build_runtime_report;
use types::RuntimeReport;

const DEFAULT_RUNTIME_REPORT_PATH: &str = "/var/lib/cynic/reports/runtime-report.json";
const DEFAULT_RUNTIME_REPORT_DIR: &str = "/var/lib/cynic/reports";
const FALLBACK_RUNTIME_REPORT_DIR: &str = "/tmp/cynic/reports";
const DEFAULT_RUNTIME_REPORT_FILENAME: &str = "runtime-report.json";

pub(crate) fn execute_report(json: bool, output: Option<&str>, persist: bool) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }

    match build_runtime_report(DEFAULT_RUNTIME_REPORT_PATH) {
        Ok(mut report) => {
            let report_dir = std::env::var("CYNIC_REPORT_DIR").ok();
            let uses_default_dir = report_dir.is_none();
            let is_output_explicit = output.is_some();
            let output_path = resolve_runtime_report_path(output, persist, report_dir.as_deref());

            if let Some(path) = output_path.as_ref() {
                if let Err(error) = write_runtime_report(path, &mut report) {
                    if is_output_explicit || !uses_default_dir {
                        return fail(error);
                    }

                    let fallback_path = Path::new(FALLBACK_RUNTIME_REPORT_DIR)
                        .join(DEFAULT_RUNTIME_REPORT_FILENAME);
                    if let Err(fallback_error) = write_runtime_report(&fallback_path, &mut report) {
                        return fail(format!(
                            "{error}; fallback {} failed: {fallback_error}",
                            fallback_path.display()
                        ));
                    }
                    eprintln!(
                        "warning: default runtime report path {} is not writable, falling back to {}",
                        path.display(),
                        fallback_path.display()
                    );
                }
            }

            emit_json(&report, false)
        }
        Err(error) => fail(error),
    }
}

fn resolve_runtime_report_path(
    output: Option<&str>,
    persist: bool,
    report_dir: Option<&str>,
) -> Option<PathBuf> {
    output.map(PathBuf::from).or_else(|| {
        persist.then(|| {
            Path::new(report_dir.unwrap_or(DEFAULT_RUNTIME_REPORT_DIR))
                .join(DEFAULT_RUNTIME_REPORT_FILENAME)
        })
    })
}

fn write_runtime_report(path: &Path, report: &mut RuntimeReport) -> Result<(), String> {
    report.persistence.written_path = Some(path.display().to_string());
    report.persistence.mutates_filesystem = true;
    write_pretty_json_file(path, report, "runtime report")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::resolve_runtime_report_path;
    use super::{
        DEFAULT_RUNTIME_REPORT_DIR, DEFAULT_RUNTIME_REPORT_FILENAME, DEFAULT_RUNTIME_REPORT_PATH,
    };
    use std::path::Path;

    #[test]
    fn default_runtime_report_path_is_installed_state_not_user_home() {
        assert_eq!(
            DEFAULT_RUNTIME_REPORT_PATH,
            "/var/lib/cynic/reports/runtime-report.json"
        );
        assert!(!DEFAULT_RUNTIME_REPORT_PATH.contains("/home/"));
        assert!(!DEFAULT_RUNTIME_REPORT_PATH.contains("~"));
    }

    #[test]
    fn default_report_filename_and_dir_are_consistent() {
        let expected_default_path =
            format!("{DEFAULT_RUNTIME_REPORT_DIR}/{DEFAULT_RUNTIME_REPORT_FILENAME}");
        assert_eq!(expected_default_path, DEFAULT_RUNTIME_REPORT_PATH);
    }

    #[test]
    fn runtime_report_path_uses_explicit_output_first() {
        let explicit = "/tmp/explicit-runtime-report.json";
        let path = resolve_runtime_report_path(Some(explicit), true, Some("/opt/custom"));
        assert_eq!(path.as_deref().unwrap(), Path::new(explicit));
        let path = resolve_runtime_report_path(Some(explicit), false, None);
        assert_eq!(path.as_deref().unwrap(), Path::new(explicit));
    }

    #[test]
    fn runtime_report_path_uses_default_directory_with_persist() {
        let path = resolve_runtime_report_path(None, true, Some("/custom/reports"));
        assert_eq!(
            path.unwrap(),
            Path::new("/custom/reports").join("runtime-report.json")
        );
    }

    #[test]
    fn runtime_report_path_does_not_allocate_when_not_persisting() {
        let path = resolve_runtime_report_path(None, false, Some("/custom/reports"));
        assert!(path.is_none());
    }
}
