use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cynic_host::runtime_network_preflight::RuntimeNetworkPreflightReport;
use serde::Serialize;

const SCHEMA: &str = "cynic.dev_check.v1";
const DEFAULT_STATE_ROOT: &str = "/var/lib/cynic";
const OUTPUT_TAIL_LIMIT: usize = 12_000;
const CARGO_FETCH_TARGETS: &[&str] = &["index.crates.io", "crates.io", "static.crates.io"];
const CANONICAL_GATE: &str = "make check";
const ARCHITECTURE_BOUNDARY_GATE: &str = "cargo test -q -p cynic-cli --test architecture_boundary";
/// Layered sovereign gate: validate each CYNIC layer in isolation and in
/// dependency order before promoting the next. `cynic-protocol` is validated
/// first because every other crate depends on it; a desync in the offline
/// cache surfaces here, not as a silent downstream build failure.
const LAYERED_GATE: &str = "layered cargo check/test by crate (protocol→host→node→kernel→mcp→cli)";
/// Canonical CYNIC crate order for isolated layer validation. Each layer is
/// `cargo check -p <crate>` then `cargo test -p <crate>` with its own isolated
/// target dir, fail-closed on the first failure (see `stop_on_first_failure`).
const LAYER_ORDER: &[&str] = &[
    "cynic-protocol",
    "cynic-host",
    "cynic-node",
    "cynic-kernel",
    "cynic-mcp",
    "cynic-cli",
];
/// Offline cache integrity probe: resolve the workspace metadata with the same
/// `--offline --locked` policy as the sovereign build, BEFORE any compile.
/// A stale `.cargo-home` registry cache (source drifted from Cargo.lock) fails
/// here with `offline_dependency_unavailable` instead of wasting a full build.
const CACHE_INTEGRITY_STEP_ID: &str = "cache_integrity";

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
    pub(crate) architecture_only: bool,
    pub(crate) layered: bool,
    pub(crate) architecture_boundary_ok: bool,
    pub(crate) architecture_boundary_fallback_ok: bool,
    pub(crate) steps: Vec<DevCheckStepReport>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DevCheckStepReport {
    pub(crate) id: String,
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
    id: String,
    program: &'static str,
    args: Vec<&'static str>,
    target_dir: Option<PathBuf>,
}

enum DevCheckMode {
    Canonical,
    ArchitectureOnly,
    Layered,
}

impl DevCheckMode {
    fn gate(&self) -> &'static str {
        match self {
            Self::Canonical => CANONICAL_GATE,
            Self::ArchitectureOnly => ARCHITECTURE_BOUNDARY_GATE,
            Self::Layered => LAYERED_GATE,
        }
    }
}

pub(crate) fn run_dev_check(
    source_root: Option<PathBuf>,
    target_root: Option<PathBuf>,
    architecture_only: bool,
    layered: bool,
) -> Result<DevCheckReport, String> {
    let workspace_root = crate::rust_source::resolve_workspace_root(source_root.as_deref())?;
    let target_root = match target_root {
        Some(target_root) => target_root,
        None => default_target_root()?,
    };
    let mode = if architecture_only {
        DevCheckMode::ArchitectureOnly
    } else if layered {
        DevCheckMode::Layered
    } else {
        DevCheckMode::Canonical
    };
    let cargo_fetch_preflight =
        cynic_host::runtime_network_preflight::observe_runtime_network_preflight(
            &CARGO_FETCH_TARGETS
                .iter()
                .map(|target| (*target).to_string())
                .collect::<Vec<_>>(),
        );
    // One living gate only — same as MCP cynic_validate and AGENTS.md.
    // Optional target_root isolates make/cargo artifacts from shared worktree target/.
    let planned = planned_steps(&target_root, &mode);
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

    let architecture_boundary_fallback_ok = if matches!(mode, DevCheckMode::Canonical)
        && failed
        && steps
            .last()
            .map(should_run_architecture_boundary_fallback)
            .unwrap_or(false)
    {
        let fallback_report = run_step(
            &workspace_root,
            architecture_boundary_step(&target_root, "architecture_boundary_fallback"),
        );
        let ok = fallback_report.status == "ok";
        steps.push(fallback_report);
        ok
    } else {
        false
    };

    let executed_count = steps.len();
    let (authority_json_ok, cargo_check_ok, surface_audit_ok) =
        if matches!(mode, DevCheckMode::Canonical) {
            steps
                .first()
                .map(|step| {
                    infer_make_check_steps(
                        &step.stdout_tail,
                        &step.stderr_tail,
                        step.status == "ok",
                    )
                })
                .unwrap_or((false, false, false))
        } else {
            (false, false, false)
        };
    let architecture_boundary_ok = steps
        .iter()
        .any(|step| is_architecture_boundary_step(step.id) && step.status == "ok");

    Ok(DevCheckReport {
        schema: SCHEMA,
        status: if failed { "blocked" } else { "ok" },
        gate: mode.gate(),
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
        architecture_only,
        layered,
        architecture_boundary_ok,
        architecture_boundary_fallback_ok,
        steps,
    })
}

fn default_target_root() -> Result<PathBuf, String> {
    let state_root = std::env::var_os("CYNIC_STATE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STATE_ROOT));
    let workload_id =
        std::env::var_os("CYNIC_BUILD_WORKLOAD_ID").unwrap_or_else(|| "unmanaged".into());
    let workload_path = Path::new(&workload_id);
    if !is_single_path_component(workload_path) {
        return Err("invalid_cynic_build_workload_id".to_string());
    }
    Ok(default_target_root_for(&state_root, workload_path))
}

fn default_target_root_for(state_root: &Path, workload_id: &Path) -> PathBuf {
    state_root
        .join("build-targets/bootstrap/agents")
        .join(workload_id)
        .join("dev-check")
}

fn is_single_path_component(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(std::path::Component::Normal(_))
    ) && path.components().count() == 1
}

fn planned_steps(target_root: &Path, mode: &DevCheckMode) -> Vec<DevCheckStep> {
    match mode {
        DevCheckMode::Canonical => vec![DevCheckStep {
            id: "make_check",
            program: "make",
            args: vec!["check"],
            // Isolate cargo artifacts for this gate invocation.
            target_dir: Some(target_root.join("make_check")),
        }],
        DevCheckMode::ArchitectureOnly => {
            vec![architecture_boundary_step(
                target_root,
                "architecture_boundary_primary",
            )]
        }
        DevCheckMode::Layered => {
            // Observer before specializing: prove the offline cache is coherent
            // with the workspace Cargo.lock before any compile attempts. A
            // stale registry cache (source drifted from Cargo.lock) fails here
            // with `offline_dependency_unavailable` instead of during a layer.
            let cache_integrity = DevCheckStep {
                id: CACHE_INTEGRITY_STEP_ID.to_string(),
                program: "scripts/cargo",
                args: vec!["metadata", "--offline", "--locked", "--format-version", "1"],
                target_dir: Some(target_root.join("cache_integrity")),
            };
            // Each CYNIC layer is validated in isolation and in dependency
            // order. `cynic-protocol` first (everything depends on it); each
            // layer gets a dedicated isolated target dir plus check + test
            // read-back. Fail-closed: the loop stops at the first failure.
            let mut steps = vec![cache_integrity];
            for layer in LAYER_ORDER {
                steps.push(DevCheckStep {
                    id: format!("layer_{layer}_check"),
                    program: "scripts/cargo",
                    args: vec!["check", "-p", layer, "--offline", "--locked"],
                    target_dir: Some(target_root.join(format!("layer_{layer}_check"))),
                });
                steps.push(DevCheckStep {
                    id: format!("layer_{layer}_test"),
                    program: "scripts/cargo",
                    args: vec!["test", "-p", layer, "--offline", "--locked", "-q"],
                    target_dir: Some(target_root.join(format!("layer_{layer}_test"))),
                });
            }
            steps
        }
    }
}

fn architecture_boundary_step(target_root: &Path, id: &'static str) -> DevCheckStep {
    DevCheckStep {
        id,
        program: "scripts/cargo",
        args: vec![
            "test",
            "-q",
            "-p",
            "cynic-cli",
            "--test",
            "architecture_boundary",
        ],
        target_dir: Some(target_root.join(id)),
    }
}

fn run_step(workspace_root: &Path, step: DevCheckStep) -> DevCheckStepReport {
    let command = std::iter::once(step.program.to_string())
        .chain(step.args.iter().map(|arg| (*arg).to_string()))
        .collect::<Vec<_>>();
    let mut env = vec![
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
    process.env("CARGO_INCREMENTAL", "0");
    process.env("CARGO_BUILD_JOBS", "1");
    process.env("CMAKE_BUILD_PARALLEL_LEVEL", "1");
    process.env("CYNIC_CARGO_BUILD_JOBS", "1");
    // RUST_MIN_STACK owned by check orchestrator (not .cargo/config): 8 GiB per
    // AGENS §3 + cynic-dev-build. Required for cold link-codegen under rustc
    // 1.96 (libp2p/reqwest/rustls = 47% of IR). Without this, the make_check
    // cargo run/build steps segfault (signal 11) on metadata+link emit.
    process.env("RUST_MIN_STACK", "8589934592");

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

fn should_run_architecture_boundary_fallback(step: &DevCheckStepReport) -> bool {
    step.id == "make_check"
        && step.status == "blocked"
        && step.failure_kind.as_deref() == Some("compiler_or_linker_artifact_corruption")
}

fn is_architecture_boundary_step(id: &str) -> bool {
    matches!(
        id,
        "architecture_boundary_primary" | "architecture_boundary_fallback"
    )
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
        || stderr.contains("rustc interrupted by SIGSEGV")
        || stderr.contains("ld terminated with signal 11")
        || stderr.contains("rust-lld")
        || stderr.contains("libLLVM")
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

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|instant| instant.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_target_root_is_node_state_and_workload_isolated() {
        assert_eq!(
            default_target_root_for(Path::new("/state"), Path::new("delegation-42")),
            PathBuf::from("/state/build-targets/bootstrap/agents/delegation-42/dev-check")
        );
    }

    #[test]
    fn workload_id_must_be_a_single_safe_path_component() {
        assert!(is_single_path_component(Path::new("delegation-42")));
        assert!(!is_single_path_component(Path::new("../escape")));
        assert!(!is_single_path_component(Path::new("agent/workload")));
        assert!(!is_single_path_component(Path::new("/absolute")));
    }

    #[test]
    fn planned_steps_are_single_make_check_gate() {
        let steps = planned_steps(
            Path::new("/tmp/cynic-dev-check-targets"),
            &DevCheckMode::Canonical,
        );
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "make_check");
        assert_eq!(steps[0].program, "make");
        assert_eq!(steps[0].args, vec!["check"]);
    }

    #[test]
    fn architecture_only_mode_plans_single_architecture_gate() {
        let steps = planned_steps(
            Path::new("/tmp/cynic-dev-check-targets"),
            &DevCheckMode::ArchitectureOnly,
        );
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "architecture_boundary_primary");
        assert_eq!(steps[0].program, "scripts/cargo");
        assert_eq!(
            steps[0].args,
            vec![
                "test",
                "-q",
                "-p",
                "cynic-cli",
                "--test",
                "architecture_boundary"
            ]
        );
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

    #[test]
    fn architecture_boundary_fallback_step_uses_isolated_target() {
        let step = architecture_boundary_step(
            Path::new("/tmp/cynic-dev-check-targets"),
            "architecture_boundary_fallback",
        );
        assert_eq!(step.id, "architecture_boundary_fallback");
        assert_eq!(step.program, "scripts/cargo");
        assert_eq!(
            step.args,
            vec![
                "test",
                "-q",
                "-p",
                "cynic-cli",
                "--test",
                "architecture_boundary"
            ]
        );
        assert_eq!(
            step.target_dir,
            Some(PathBuf::from(
                "/tmp/cynic-dev-check-targets/architecture_boundary_fallback"
            ))
        );
    }

    #[test]
    fn linker_crash_runs_architecture_boundary_fallback() {
        let step = DevCheckStepReport {
            id: "make_check",
            status: "blocked",
            command: vec!["make".to_string(), "check".to_string()],
            env: vec![],
            cwd: ".".to_string(),
            target_dir: Some("/tmp/cynic-dev-check-targets/make_check".to_string()),
            exit_code: Some(101),
            duration_ms: 1,
            stdout_tail: String::new(),
            stderr_tail: "rustc interrupted by SIGSEGV".to_string(),
            failure_kind: Some("compiler_or_linker_artifact_corruption".to_string()),
        };
        assert!(should_run_architecture_boundary_fallback(&step));
    }

    #[test]
    fn classify_failure_recognizes_rustc_and_linker_sigsegv() {
        assert_eq!(
            classify_failure(b"rustc interrupted by SIGSEGV"),
            "compiler_or_linker_artifact_corruption"
        );
        assert_eq!(
            classify_failure(b"collect2: fatal error: ld terminated with signal 11"),
            "compiler_or_linker_artifact_corruption"
        );
    }
}
