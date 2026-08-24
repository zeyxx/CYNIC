use std::path::Path;
use std::process::ExitCode;

use cynic_host::framework_bootstrap;
use cynic_host::update_lifecycle::LocalArgvRunner;

use crate::fs_json::write_pretty_json_file;
use crate::{emit_json, fail};

pub(super) fn execute_plan(
    json: bool,
    repository_root: String,
    source_commit: String,
    output_dir: String,
    output: Option<String>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let plan = framework_bootstrap::plan_source_snapshot(
        &repository_root,
        &source_commit,
        &output_dir,
        &LocalArgvRunner,
    );
    if let Some(out) = output {
        if let Err(error) = write_pretty_json_file(Path::new(&out), &plan, "source snapshot plan") {
            return fail(error);
        }
    }
    emit_json(&plan, plan.status == "blocked")
}

pub(super) fn execute_create(
    json: bool,
    repository_root: String,
    source_commit: String,
    output_dir: String,
    yes: bool,
    output: Option<String>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let report = framework_bootstrap::create_source_snapshot(
        &repository_root,
        &source_commit,
        &output_dir,
        yes,
        &LocalArgvRunner,
    );
    if let Some(out) = output {
        if let Err(error) =
            write_pretty_json_file(Path::new(&out), &report, "source snapshot report")
        {
            return fail(error);
        }
    }
    emit_json(&report, report.status != "proved")
}
