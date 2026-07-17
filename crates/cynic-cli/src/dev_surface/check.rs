use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cynic_host::runtime_network_preflight::RuntimeNetworkPreflightReport;
use serde::Serialize;

const SCHEMA: &str = "cynic.dev_check.v1";
const DEFAULT_TARGET_ROOT: &str = "/tmp/cynic-dev-check-targets";
const OUTPUT_TAIL_LIMIT: usize = 12_000;
const CARGO_FETCH_TARGETS: &[&str] = &["index.crates.io", "crates.io", "static.crates.io"];
const CANONICAL_GATE: &str = "make check";

#[derive(Debug, Serialize)]
pub(crate) struct DevCheckReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) gate: &'static str,
    pub(crate) generated_at_unix_ms: u128,
    pub(crate) workspace_root: String,
    pub(crate) target_root: String,
    pub(crate) mutates_filesystem: bool,
    pub(crate) mutation_boundary: &'static str,
    pub(crate) cargo_network_policy: &'static str,
    pub(crate) cargo_target_policy: CargoTargetPolicy,
    pub(crate) cargo_fetch_preflight: RuntimeNetworkPreflightReport,
    pub(crate) stop_on_first_failure: bool,
    pub(crate) planned_count: usize,
    pub(crate) executed_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) authority_json_ok: bool,
    pub(crate) cargo_check_ok: bool,
    pub(crate) surface_audit_ok: bool,
    pub(crate) steps: Vec<DevCheckStepReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DevCheckStepReport {
    pub(crate) id: &'static str,
    pub(crate) status: &'static str,
    pub(crate) command: Vec<String>,
    pub(crate) env: Vec<DevCheckEnv>,
    pub(crate) cwd: String,
    pub(crate) target_dir: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) duration_ms: u128,
    pub(crate) stdout_tail: String,
    pub(crate) stderr_tail: String,
    pub(crate) failure_kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DevCheckEnv {
    pub(crate) key: &'static str,
    pub(crate) value: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CargoTargetPolicy {
    pub(crate) isolation: &'static str,
    pub(crate) root: String,
    pub(crate) shared_worktree_target_allowed: bool,
    pub(crate) reason: &'static str,
}

struct DevCheckStep {
    id: &'static str,
    program: &'static str,
    args: Vec<&'static str>,
    target_dir: Option<PathBuf>,
}

pub(crate) fn run_dev_check(target_root: Option<PathBuf>) -> DevCheckReport {
    let workspace_root = workspace_root();
    let target_root = target_root.unwrap_or_else(|| PathBuf::from(DEFAULT_TARGET_ROOT));
    let cargo_fetch_preflight =
        cynic_host::runtime_network_preflight::observe_runtime_network_preflight(
            &CARGO_FETCH_TARGETS
                .iter()
                .map(|target| (*target).to_string())
                .collect::<Vec<_>>(),
        );
    // One living gate only — same as MCP cynic_validate and AGENTS.md.
    // Optional target_root isolates make/cargo artifacts from shared worktree target/.
    let planned = planned_steps(&target_root);
    let planned_count = planned.len();
    let mut steps = Vec::with_capacity(planned_count);
    let mut failed = false;

    for step in planned {
        if failed {
            break;
        }
        let report = run_step(&workspace_root, step);
        failed = report.status != "ok";
        steps.push(report);
    }

    let executed_count = steps.len();
    let (authority_json_ok, cargo_check_ok, surface_audit_ok) = steps
        .first()
        .map(|step| {
            infer_make_check_steps(&step.stdout_tail, &step.stderr_tail, step.status == "ok")
        })
        .unwrap_or((false, false, false));

    DevCheckReport {
        schema: SCHEMA,
        status: if failed { "blocked" } else { "ok" },
        gate: CANONICAL_GATE,
        generated_at_unix_ms: now_unix_ms(),
        workspace_root: workspace_root.display().to_string(),
        target_root: target_root.display().to_string(),
        mutates_filesystem: true,
        mutation_boundary: "build artifacts only under target_root via CARGO_TARGET_DIR; no host OS mutation",
        cargo_network_policy: "make_check_offline_default",
        cargo_target_policy: CargoTargetPolicy {
            // Keep the historical isolation token used by architecture boundary tests
            // while isolating the single make-check cargo tree under target_root.
            isolation: "per_step_target_dir",
            root: target_root.display().to_string(),
            shared_worktree_target_allowed: false,
            reason: "dev check delegates to make check but must not share worktree target/debug artifacts across agents, sessions, or native C build scripts",
        },
        cargo_fetch_preflight,
        stop_on_first_failure: true,
        planned_count,
        executed_count,
        skipped_count: planned_count.saturating_sub(executed_count),
        authority_json_ok,
        cargo_check_ok,
        surface_audit_ok,
        steps,
    }
}

fn planned_steps(target_root: &Path) -> Vec<DevCheckStep> {
    vec![DevCheckStep {
        id: "make_check",
        program: "make",
        args: vec!["check"],
        // Isolate cargo artifacts for this gate invocation.
        target_dir: Some(target_root.join("make_check")),
    }]
}

fn run_step(workspace_root: &Path, step: DevCheckStep) -> DevCheckStepReport {
    let command = std::iter::once(step.program.to_string())
        .chain(step.args.iter().map(|arg| (*arg).to_string()))
        .collect::<Vec<_>>();
    let mut env = vec![
        DevCheckEnv {
            key: "CYNIC_ALLOW_LOCAL_DEV_BUILD",
            value: "1".to_string(),
        },
        DevCheckEnv {
            key: "CARGO_INCREMENTAL",
            value: "0".to_string(),
        },
        DevCheckEnv {
            key: "CARGO_BUILD_JOBS",
            value: "1".to_string(),
        },
        DevCheckEnv {
            key: "CMAKE_BUILD_PARALLEL_LEVEL",
            value: "1".to_string(),
        },
        DevCheckEnv {
            key: "CYNIC_CARGO_BUILD_JOBS",
            value: "1".to_string(),
        },
    ];
    let mut process = Command::new(step.program);
    process.current_dir(workspace_root);
    process.args(&step.args);
    process.env("CYNIC_ALLOW_LOCAL_DEV_BUILD", "1");
    process.env("CARGO_INCREMENTAL", "0");
    process.env("CARGO_BUILD_JOBS", "1");
    process.env("CMAKE_BUILD_PARALLEL_LEVEL", "1");
    process.env("CYNIC_CARGO_BUILD_JOBS", "1");

    if let Some(target_dir) = &step.target_dir {
        process.env("CARGO_TARGET_DIR", target_dir);
        env.push(DevCheckEnv {
            key: "CARGO_TARGET_DIR",
            value: target_dir.display().to_string(),
        });
    }

    let started = Instant::now();
    match process.output() {
        Ok(output) => DevCheckStepReport {
            id: step.id,
            status: if output.status.success() {
                "ok"
            } else {
                "blocked"
            },
            command,
            env,
            cwd: workspace_root.display().to_string(),
            target_dir: step.target_dir.map(|path| path.display().to_string()),
            exit_code: output.status.code(),
            duration_ms: started.elapsed().as_millis(),
            stdout_tail: tail_lossy(&output.stdout),
            stderr_tail: tail_lossy(&output.stderr),
            failure_kind: if output.status.success() {
                None
            } else {
                Some(classify_failure(&output.stderr))
            },
        },
        Err(error) => DevCheckStepReport {
            id: step.id,
            status: "blocked",
            command,
            env,
            cwd: workspace_root.display().to_string(),
            target_dir: step.target_dir.map(|path| path.display().to_string()),
            exit_code: None,
            duration_ms: started.elapsed().as_millis(),
            stdout_tail: String::new(),
            stderr_tail: error.to_string(),
            failure_kind: Some("spawn_failure".to_string()),
        },
    }
}

fn infer_make_check_steps(stdout: &str, stderr: &str, passed: bool) -> (bool, bool, bool) {
    if passed {
        return (true, true, true);
    }
    let combined = format!("{stdout}\n{stderr}");
    let cargo_started = combined.contains("cargo check core crates");
    let audit_started = combined.contains("auditing canonical CLI surfaces");
    let authority_json_ok = cargo_started || audit_started;
    let cargo_check_ok = audit_started;
    let surface_audit_ok = false;
    (authority_json_ok, cargo_check_ok, surface_audit_ok)
}

fn classify_failure(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    if stderr.contains("Could not resolve host") || stderr.contains("Could not resolve hostname") {
        "network_dns_resolution".to_string()
    } else if stderr.contains("network is unreachable") || stderr.contains("failed to download") {
        "network_unavailable".to_string()
    } else if stderr.contains("signal: 11")
        || stderr.contains("SIGSEGV")
        || stderr.contains("invalid symbol index")
        || stderr.contains("internal compiler error: Segmentation fault")
        || stderr.contains("error occurred in cc-rs")
    {
        "compiler_or_linker_artifact_corruption".to_string()
    } else if stderr.contains("no matching package") || stderr.contains("not found in registry") {
        "offline_dependency_unavailable".to_string()
    } else {
        "process_exit_failure".to_string()
    }
}

fn tail_lossy(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(OUTPUT_TAIL_LIMIT);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("cynic-cli must live under crates/cynic-cli")
        .to_path_buf()
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must be after unix epoch")
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_steps_are_single_make_check_gate() {
        let steps = planned_steps(Path::new("/tmp/cynic-dev-check-targets"));
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "make_check");
        assert_eq!(steps[0].program, "make");
        assert_eq!(steps[0].args, vec!["check"]);
    }

    #[test]
    fn infer_steps_fail_closed_before_cargo() {
        let (a, c, s) = infer_make_check_steps(
            "==> checking foundation, runtime authority, and registry\n",
            "jq: parse error",
            false,
        );
        assert!(!a);
        assert!(!c);
        assert!(!s);
    }
}
