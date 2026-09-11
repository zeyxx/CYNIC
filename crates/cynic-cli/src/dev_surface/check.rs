use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cynic_host::runtime_network_preflight::RuntimeNetworkPreflightReport;
use serde::Serialize;

const SCHEMA: &str = "cynic.dev_check.v1";
const DEFAULT_STATE_ROOT: &str = "/var/lib/cynic";
const OUTPUT_TAIL_LIMIT: usize = 12_000;
const CARGO_FETCH_TARGETS: &[&str] = &["index.crates.io", "crates.io", "static.crates.io"];
const CANONICAL_GATE: &str = "cynic dev check (native)";
const ARCHITECTURE_BOUNDARY_GATE: &str = "cargo test -q -p cynic-cli --test architecture_boundary";

/// Native canonical crate order. Dependency direction, workspace members only.
/// cynic-askesis is excluded: the directory does not exist (contract drift,
/// see runtime-authority audit). cynic-mcp is excluded: no such workspace
/// member. This list must stay aligned with Cargo.toml workspace members.

/// Dependency-ordered validation layers. Each layer is checked and tested in
/// isolation before the next, so a break in one crate does not mask or cascade
/// into the others. Order = compilation/dependency direction.
const NATIVE_CHECK_CRATES: &[&str] = &[
    "cynic-protocol",
    "cynic-host",
    "cynic-node",
    "cynic-kernel",
    "cynic-cli",
    "cynic-inference",
];
const LAYER_ORDER: &[&str] = &[
    "cynic-protocol",
    "cynic-host",
    "cynic-node",
    "cynic-kernel",
    "cynic-cli",
];
const CACHE_INTEGRITY_STEP_ID: &str = "cache_integrity_offline_locked";

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
    pub(crate) architecture_boundary_ok: bool,
    pub(crate) architecture_boundary_fallback_ok: bool,
    pub(crate) layered: bool,
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
            Self::Layered => "make check-layers",
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
    let mode = if layered {
        DevCheckMode::Layered
    } else if architecture_only {
        DevCheckMode::ArchitectureOnly
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
            architecture_boundary_step(&target_root, "architecture_boundary_fallback".to_string()),
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
            aggregate_canonical_steps(&steps)
        } else {
            (false, false, false)
        };
    let architecture_boundary_ok = steps
        .iter()
        .any(|step| is_architecture_boundary_step(&step.id) && step.status.eq("ok"));

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
            reason: "native dev check isolates cargo artifacts under target_root; never share worktree target/debug across agents, sessions, or native C build scripts",
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
        architecture_boundary_ok,
        architecture_boundary_fallback_ok,
        layered,
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
        // Native canonical gate: no make, no jq, no shell pipeline.
        // Authority JSON is validated in-process, cargo check runs per crate
        // with an isolated CARGO_TARGET_DIR, surface audit calls the real
        // audit function directly, and the installed-runtime guard runs as a
        // cargo test step. Exit code of `cynic dev check --json` is the gate.
        DevCheckMode::Canonical => {
            let mut steps = vec![DevCheckStep {
                id: "authority_json".to_string(),
                program: "native:authority_json",
                args: vec![],
                target_dir: None,
            }];
            for krate in NATIVE_CHECK_CRATES {
                steps.push(DevCheckStep {
                    id: format!("cargo_check_{}", krate.replace('-', "_")),
                    program: "cargo",
                    args: vec!["check", "-p", krate],
                    target_dir: Some(target_root.join("native_check")),
                });
            }
            steps.push(DevCheckStep {
                id: "surface_audit".to_string(),
                program: "native:surface_audit",
                args: vec![],
                target_dir: None,
            });
            steps.push(DevCheckStep {
                id: "installed_runtime".to_string(),
                program: "cargo",
                args: vec![
                    "test",
                    "-q",
                    "-p",
                    "cynic-cli",
                    "--test",
                    "installed_runtime",
                ],
                target_dir: Some(target_root.join("installed_runtime")),
            });
            steps
        }
        DevCheckMode::ArchitectureOnly => {
            vec![architecture_boundary_step(
                target_root,
                "architecture_boundary_primary".to_string(),
            )]
        }
        DevCheckMode::Layered => {
            let mut steps = vec![DevCheckStep {
                id: CACHE_INTEGRITY_STEP_ID.to_string(),
                program: "cargo",
                args: vec!["metadata", "--offline", "--locked", "--format-version", "1"],
                target_dir: Some(target_root.join("cache_integrity")),
            }];
            for layer in LAYER_ORDER {
                steps.push(DevCheckStep {
                    id: format!("check_layer_{}", layer),
                    program: "cargo",
                    args: vec!["check", "-p", layer],
                    target_dir: Some(target_root.join(format!("layer_{}", layer))),
                });
            }
            steps
        }
    }
}

fn architecture_boundary_step(target_root: &Path, id: String) -> DevCheckStep {
    DevCheckStep {
        id: id.clone(),
        program: "cargo",
        args: vec![
            "test",
            "-q",
            "-p",
            "cynic-cli",
            "--test",
            "architecture_boundary",
        ],
        target_dir: Some(target_root.join(&id)),
    }
}

fn run_step(workspace_root: &Path, step: DevCheckStep) -> DevCheckStepReport {
    if step.program == "native:authority_json" {
        return run_native_authority_json(workspace_root, step);
    }
    if step.program == "native:surface_audit" {
        return run_native_surface_audit(workspace_root, step);
    }
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
    if let Ok(toolchain) = std::env::var("RUSTUP_TOOLCHAIN") {
        process.env("RUSTUP_TOOLCHAIN", &toolchain);
        env.push(DevCheckEnv {
            key: "RUSTUP_TOOLCHAIN",
            value: toolchain,
        });
    }
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

/// Native authority validation: parse the four machine-readable authorities
/// in-process. Replaces `jq empty` from make authority-check. No subprocess,
/// no shell, no jq.
fn run_native_authority_json(workspace_root: &Path, step: DevCheckStep) -> DevCheckStepReport {
    const AUTHORITY_FILES: &[&str] = &[
        "contracts/foundation.json",
        "infra/runtime-authority.json",
        "infra/registry.json",
        "infra/agent-exposure.json",
    ];
    let started = Instant::now();
    let command = vec!["native:authority_json".to_string()];
    let mut failures = Vec::new();
    for relative in AUTHORITY_FILES {
        let path = workspace_root.join(relative);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                if let Err(error) = serde_json::from_str::<serde_json::Value>(&content) {
                    failures.push(format!("{relative}: invalid JSON: {error}"));
                }
            }
            Err(error) => failures.push(format!("{relative}: cannot read: {error}")),
        }
    }
    let ok = failures.is_empty();
    DevCheckStepReport {
        id: step.id,
        status: if ok { "ok" } else { "blocked" },
        command,
        env: vec![],
        cwd: workspace_root.display().to_string(),
        target_dir: None,
        exit_code: Some(if ok { 0 } else { 1 }),
        duration_ms: started.elapsed().as_millis(),
        stdout_tail: if ok {
            "authority_json: 4 files parsed".to_string()
        } else {
            String::new()
        },
        stderr_tail: failures.join("\n"),
        failure_kind: if ok {
            None
        } else {
            Some("authority_json_invalid".to_string())
        },
    }
}

/// Native surface audit: call the real audit function directly instead of
/// spawning `make surface-audit` and piping through jq.
fn run_native_surface_audit(workspace_root: &Path, step: DevCheckStep) -> DevCheckStepReport {
    let started = Instant::now();
    let report = crate::surface_audit::audit_canonical_surfaces();
    let ok = report.blocked_count == 0;
    let mut stderr_tail = String::new();
    if !ok {
        let mut lines = Vec::new();
        for surface in &report.surfaces {
            if surface.status != "ok" {
                lines.push(format!("surface {}: {}", surface.id, surface.status));
            }
        }
        for finding in &report.external_projection_findings {
            if finding.status != "ok" {
                lines.push(format!(
                    "projection {}: {}",
                    finding.file_path, finding.status
                ));
            }
        }
        if report.agent_exposure.status != "ok" {
            lines.push(format!(
                "agent_exposure: {} blocked",
                report.agent_exposure.blocked_count
            ));
        }
        stderr_tail = lines.join("\n");
    }
    DevCheckStepReport {
        id: step.id,
        status: if ok { "ok" } else { "blocked" },
        command: vec!["native:surface_audit".to_string()],
        env: vec![],
        cwd: workspace_root.display().to_string(),
        target_dir: None,
        exit_code: Some(if ok { 0 } else { 1 }),
        duration_ms: started.elapsed().as_millis(),
        stdout_tail: format!(
            "surface_audit: {}/{} ok, {} blocked",
            report.ok_count, report.surface_count, report.blocked_count
        ),
        stderr_tail,
        failure_kind: if ok {
            None
        } else {
            Some("surface_audit_blocked".to_string())
        },
    }
}

/// Aggregate native canonical steps into the three report flags. No stdout
/// scraping: each step carries its own typed status.
fn aggregate_canonical_steps(steps: &[DevCheckStepReport]) -> (bool, bool, bool) {
    let by_id = |id: &str| steps.iter().find(|step| step.id == id);
    let authority_json_ok = by_id("authority_json")
        .map(|step| step.status == "ok")
        .unwrap_or(false);
    let cargo_check_ok = NATIVE_CHECK_CRATES.iter().all(|krate| {
        let id = format!("cargo_check_{}", krate.replace('-', "_"));
        by_id(&id).map(|step| step.status == "ok").unwrap_or(false)
    });
    let surface_audit_ok = by_id("surface_audit")
        .map(|step| step.status == "ok")
        .unwrap_or(false);
    (authority_json_ok, cargo_check_ok, surface_audit_ok)
}

fn should_run_architecture_boundary_fallback(step: &DevCheckStepReport) -> bool {
    step.id.starts_with("cargo_check_")
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
    fn planned_steps_are_native_without_make_or_jq() {
        let steps = planned_steps(
            Path::new("/tmp/cynic-dev-check-targets"),
            &DevCheckMode::Canonical,
        );
        assert_eq!(steps.len(), NATIVE_CHECK_CRATES.len() + 3);
        assert_eq!(steps[0].id, "authority_json");
        assert_eq!(steps[0].program, "native:authority_json");
        assert!(steps.iter().all(|step| step.program != "make"));
        let cargo_ids: Vec<&str> = steps
            .iter()
            .filter(|step| step.program == "cargo")
            .map(|step| step.id.as_str())
            .collect();
        assert_eq!(cargo_ids.len(), NATIVE_CHECK_CRATES.len() + 1);
        let last_native = steps
            .iter()
            .find(|step| step.id == "surface_audit")
            .expect("surface_audit step");
        assert_eq!(last_native.program, "native:surface_audit");
    }

    #[test]
    fn architecture_only_mode_plans_single_architecture_gate() {
        let steps = planned_steps(
            Path::new("/tmp/cynic-dev-check-targets"),
            &DevCheckMode::ArchitectureOnly,
        );
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "architecture_boundary_primary");
        assert_eq!(steps[0].program, "cargo");
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
    fn aggregate_steps_fail_closed_without_all_native_ok() {
        let steps: Vec<DevCheckStepReport> = Vec::new();
        let (a, c, s) = aggregate_canonical_steps(&steps);
        assert!(!a);
        assert!(!c);
        assert!(!s);
    }

    #[test]
    fn architecture_boundary_fallback_step_uses_isolated_target() {
        let step = architecture_boundary_step(
            Path::new("/tmp/cynic-dev-check-targets"),
            "architecture_boundary_fallback".to_string(),
        );
        assert_eq!(step.id, "architecture_boundary_fallback");
        assert_eq!(step.program, "cargo");
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
            id: "cargo_check_cynic_kernel".to_string(),
            status: "blocked",
            command: vec!["cargo".to_string(), "check".to_string()],
            env: vec![],
            cwd: ".".to_string(),
            target_dir: Some("/tmp/cynic-dev-check-targets/native_check".to_string()),
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
