//! Build triggering, execution and proof verification.

use cynic_build::build::{BUILD_RECIPE_SCHEMA, BuildRecipe, canonical_argv_from_build_action};
use cynic_build::build_proof::{BuildProofPhase, BuildProofRequest};
use cynic_host::build_evidence::{BuildRunnerObservationPort, JsonBuildRunnerObservationAdapter};
use cynic_host::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};
use cynic_lifecycle::LocalArgvRunner;
use cynic_node::ports::build_runner::{BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode};
use cynic_vcs::SourceSnapshot;

pub(crate) fn resolve_default_builder_url() -> Result<String, String> {
    cynic_build::resolve_builder_url_with_health()
}

/// Classify a commit's diff into the minimal gate (local `git diff`,
/// read-only). Returns the scope plus the changed paths (for fmt checks).
/// Unresolvable diff fails open to Full: never skip silently.
pub(crate) fn resolve_gate_scope(
    commit: &str,
) -> (cynic_build::gate_scope::GateScope, Vec<String>) {
    use cynic_build::gate_scope::{GateScope, decide_gate_scope};
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{commit}^"), commit])
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let paths: Vec<&str> = text.lines().collect();
            let owned: Vec<String> = paths.iter().map(|p| p.to_string()).collect();
            (decide_gate_scope(&paths), owned)
        }
        _ => (
            GateScope::Full {
                reason: "commit diff unresolvable locally".to_string(),
            },
            Vec::new(),
        ),
    }
}

/// Format gate on the touched Rust files (standalone rustfmt, no cargo:
/// instant, negligible RAM). Red blocks before any builder second burns.
pub(crate) fn check_rustfmt(paths: &[String]) -> Result<(), String> {
    let rs: Vec<&String> = paths
        .iter()
        .filter(|p| p.ends_with(".rs") && !p.contains("target/"))
        .collect();
    if rs.is_empty() {
        return Ok(());
    }
    let status = std::process::Command::new("rustfmt")
        .arg("--check")
        .args(rs)
        .status()
        .map_err(|e| format!("rustfmt unavailable ({e}); format unchecked"))?;
    if status.success() {
        eprintln!("gate-scope: rustfmt clean on {} files", paths.len());
        Ok(())
    } else {
        Err("rustfmt --check red: run rustfmt on the touched files".to_string())
    }
}

/// Consumers lookup over infra/crate-topology.json (sibling of the
/// registry file). Missing/unparsable topology fails open to Full.
pub(crate) fn topology_consumers(
    registry: &str,
) -> Option<impl Fn(&str) -> Vec<String>> {
    let path = std::path::Path::new(registry)
        .parent()
        .map(|parent| parent.join("crate-topology.json"))?;
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let crates = value.get("crates")?.as_object()?.clone();
    Some(move |name: &str| {
        crates
            .get(name)
            .and_then(|c| c.get("consumers"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Run one apps/<suite> pytest tree locally (no builder dispatch).
/// Uses the same ephemeral runner as the data contracts (uvx).
pub(crate) fn run_python_suite(suite: &str) -> Result<(), String> {
    eprintln!("gate-scope: python suite {suite} (local, no builder dispatch)");
    let status = std::process::Command::new("uvx")
        .args([
            "--with",
            "pytest",
            "python",
            "-m",
            "pytest",
            &format!("apps/{suite}/tests/"),
            "-q",
            "-p",
            "no:cacheprovider",
        ])
        .status()
        .map_err(|e| format!("pytest runner unavailable: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("python suite {suite} failed"))
    }
}

/// Changed JSON files are still well-formed. No compile, no tests.
pub(crate) fn validate_json_files(files: &[String]) -> Result<(), String> {
    for file in files {
        let raw =
            std::fs::read_to_string(file).map_err(|e| format!("{file} unreadable: {e}"))?;
        serde_json::from_str::<serde_json::Value>(&raw)
            .map_err(|e| format!("{file} invalid JSON: {e}"))?;
        eprintln!("gate-scope: valid {file}");
    }
    Ok(())
}

/// Median duration over the last 20 converged proofs for (node, action)
/// in a proof-log text, with the sample count. Malformed lines are
/// skipped (a corrupt log line must not poison placement). No history
/// is (None, 0): unexplored, never instant.
pub(crate) fn proof_median_ms(log_text: &str, node: &str, action: &str) -> (Option<u64>, usize) {
    let mut durations: Vec<u64> = log_text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| {
            record.get("node").and_then(|v| v.as_str()) == Some(node)
                && record.get("action").and_then(|v| v.as_str()) == Some(action)
                && record.get("status").and_then(|v| v.as_str()) == Some("converged")
        })
        .filter_map(|record| record.get("duration_ms").and_then(|v| v.as_u64()))
        .collect();
    // Most-recent-first: the log appends, so the tail is the freshest.
    durations.reverse();
    durations.truncate(20);
    let samples = durations.len();
    if samples == 0 {
        return (None, 0);
    }
    durations.sort_unstable();
    let median = if samples % 2 == 1 {
        durations[samples / 2]
    } else {
        (durations[samples / 2 - 1] + durations[samples / 2]) / 2
    };
    (Some(median), samples)
}

pub(crate) fn submit_build(
    commit: &str,
    package: Option<&str>,
    action: &str,
    requested_by: &str,
    builder_url: &str,
    wait: bool,
    json: bool,
) -> Result<(), String> {
    // Package scope is honored end-to-end. The trigger carries it into
    // `cargo --package` on the runner and the proof carries it into
    // check/test. None means workspace. There is no ignored-scope path.
    let _ = package;
    let proof =
        cynic_build::submit_workload(commit, package, action, requested_by, builder_url, wait)?;
    if wait {
        // Terminal envelope only: an acceptance proof is not a result and must
        // never land in the proof log. Persist the waited terminal state so
        // `build proofs list --commit` reads submit results like trigger ones.
        match crate::build_proof_log::default_proof_log_path() {
            Ok(log_path) => {
                let rendered = serde_json::to_string(&proof).unwrap_or_default();
                if let Err(error) = crate::build_proof_log::append_proof(&log_path, &rendered) {
                    eprintln!("warning: proof retained in stdout only: {error}");
                }
            }
            Err(error) => eprintln!("warning: proof retained in stdout only: {error}"),
        }
    }
    cynic_build::render_workload_proof(&proof, json)?;
    if wait {
        // Terminal envelope only: an acceptance proof is not a result and must
        // never land in the proof log. Persist the waited terminal state so
        // `build proofs list --commit` reads submit results like trigger ones.
        let completion_status = proof
            .get("completion")
            .and_then(|c| c.get("status"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        if completion_status != "completed" && !completion_status.is_empty() {
            report_failure_output(&proof, builder_url);
            return Err(format!("build {completion_status}"));
        }
        if completion_status == "completed" {
            if let Some(wid) = proof.get("workload_id").and_then(|w| w.as_str()) {
                if let Ok(record) = cynic_build::fetch_workload_record(builder_url, wid) {
                    if let Some(exit_code) = record
                        .get("completion")
                        .and_then(|c| c.get("exit_code"))
                        .and_then(|e| e.as_i64())
                    {
                        if exit_code != 0 {
                            report_failure_output(&proof, builder_url);
                            return Err(format!("build failed with exit code {exit_code}"));
                        }
                    }
                }
            }
        }
        report_failure_output(&proof, builder_url);
    }
    Ok(())
}

/// Lines of sealed output shown after a failed `submit --wait`.
const FAILURE_TAIL_LINES: usize = 40;

/// After a terminal non-success, point at the sealed output on stderr so the
/// proof on stdout stays machine-readable. Never fails the submit: the proof
/// is already rendered and persisted.
fn report_failure_output(proof: &serde_json::Value, builder_url: &str) {
    let status = proof
        .get("completion")
        .and_then(|completion| completion.get("status"))
        .and_then(|status| status.as_str())
        .unwrap_or_default();
    if status == "completed" || status.is_empty() {
        return;
    }
    let Some(workload_id) = proof.get("workload_id").and_then(|id| id.as_str()) else {
        return;
    };
    match cynic_build::fetch_workload_record(builder_url, workload_id) {
        Ok(record) => match cynic_build::failure_output_from_record(&record, FAILURE_TAIL_LINES) {
            Some(output) => {
                eprintln!(
                    "workload {workload_id} {status}: exit_code={:?}; last {FAILURE_TAIL_LINES} lines of stderr follow \
                     (full: cynic build workload output --workload-id {workload_id})",
                    output.exit_code
                );
                eprintln!("{}", output.stderr_tail);
            }
            None => eprintln!("workload {workload_id} {status}: no sealed output in the record"),
        },
        Err(error) => eprintln!("workload {workload_id} {status}: output unreadable: {error}"),
    }
}

/// `build workload output`: print the output a builder sealed for a workload.
pub(crate) fn workload_output(
    workload_id: &str,
    builder_url: &str,
    tail: usize,
    json: bool,
) -> Result<(), String> {
    let record = cynic_build::fetch_workload_record(builder_url, workload_id)?;
    let output = cynic_build::failure_output_from_record(&record, tail).ok_or_else(|| {
        format!(
            "workload {workload_id} sealed no output (a successful build records none, or the id is unknown)"
        )
    })?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
        );
    } else {
        println!("exit_code: {:?}", output.exit_code);
        println!("--- stdout (last {tail} lines)");
        println!("{}", output.stdout_tail);
        println!("--- stderr (last {tail} lines)");
        println!("{}", output.stderr_tail);
    }
    Ok(())
}

/// Read the admin secret from the environment. Never from argv: process
/// lists leak argv to every local user, and a secret in shell history
/// outlives the signal.
fn admin_secret() -> Result<String, String> {
    std::env::var("CYNIC_ADMIN_SECRET")
        .map_err(|_| "CYNIC_ADMIN_SECRET is not set; export it before signaling".to_string())
        .and_then(|secret| {
            if secret.trim().is_empty() {
                Err("CYNIC_ADMIN_SECRET is empty; refusing an unsigned signal".to_string())
            } else {
                Ok(secret)
            }
        })
}

fn render_admin_reply(reply: &serde_json::Value, json: bool) -> Result<(), String> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(reply).map_err(|error| error.to_string())?
        );
    } else {
        println!(
            "{}",
            reply
                .get("outcome")
                .and_then(|outcome| outcome.as_str())
                .unwrap_or("signaled")
        );
    }
    Ok(())
}

/// Send a drain signal to a builder node: stop admitting, finish
/// in-flight, then exit for supervised relaunch.
pub(crate) fn drain_node(builder_url: &str, json: bool) -> Result<(), String> {
    let secret = admin_secret()?;
    let reply = cynic_build::admin_signal::send_drain(builder_url, &secret)?;
    render_admin_reply(&reply, json)
}

/// Send a restart signal to a builder node. The node bounces only when
/// the installed binary already matches `desired_sha256`.
pub(crate) fn restart_node(
    builder_url: &str,
    desired_sha256: &str,
    json: bool,
) -> Result<(), String> {
    let secret = admin_secret()?;
    let reply = cynic_build::admin_signal::send_restart(builder_url, &secret, desired_sha256)?;
    render_admin_reply(&reply, json)
}

/// Package scope flows into the builder payload and the runner honors it
/// (`cargo --package` in the trigger adapter, `--package` plus `--lib` in
/// the proof adapter). This predicate exists only to name that rule.

pub(crate) fn trigger_build(
    commit: &str,
    phase: &str,
    host: Option<&str>,
    runner: Option<&str>,
    registry: &str,
    user: &str,
    network_mode: &str,
    checkout_base: &str,
    ssot_url: &str,
    package: Option<String>,
    scope: &str,
    sccache_dir: &str,
    proof_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut package = package;
    match scope {
        "full" => {}
        "auto" => match resolve_gate_scope(commit) {
            (cynic_build::gate_scope::GateScope::Nothing { reason }, _) => {
                println!("gate-scope: nothing to prove ({reason})");
                return Ok(());
            }
            (cynic_build::gate_scope::GateScope::PythonSuite { suite }, _) => {
                return run_python_suite(&suite);
            }
            (cynic_build::gate_scope::GateScope::ValidateOnly { files }, _) => {
                return validate_json_files(&files);
            }
            (cynic_build::gate_scope::GateScope::RustCheck { packages }, paths) => {
                check_rustfmt(&paths)?;
                if package.is_none() {
                    match topology_consumers(registry) {
                        Some(consumers) => {
                            let closed = cynic_build::gate_scope::expand_with_consumers(
                                &packages, &consumers,
                            );
                            if closed.len() > cynic_build::gate_scope::CLOSURE_CAP {
                                eprintln!(
                                    "gate-scope: full (closure {} > cap {})",
                                    closed.len(),
                                    cynic_build::gate_scope::CLOSURE_CAP
                                );
                            } else {
                                eprintln!("gate-scope: narrowing check to {}", closed.join(","));
                                package = Some(closed.join(","));
                            }
                        }
                        None => {
                            if packages.len() == 1 {
                                eprintln!("gate-scope: narrowing check to {}", packages[0]);
                                package = Some(packages[0].clone());
                            } else {
                                eprintln!("gate-scope: full (no topology, multi-crate)");
                            }
                        }
                    }
                }
            }
            (cynic_build::gate_scope::GateScope::Full { reason }, _) => {
                eprintln!("gate-scope: full ({reason})");
            }
        },
        other => return Err(format!("invalid gate scope: {other} (full|auto)")),
    }
    let auto_route = host.as_ref().is_none_or(|v| v.trim().is_empty())
        && runner.as_ref().is_none_or(|v| v.trim().is_empty());
    let host = match host.filter(|value| !value.trim().is_empty()) {
        Some(explicit) => explicit.to_string(),
        None => match runner.filter(|value| !value.trim().is_empty()) {
            Some(named) => cynic_host::registry::resolve::list_runner_nodes(Some(registry))?
                .into_iter()
                .find(|candidate| candidate.node_id == named)
                .map(|candidate| candidate.ssh_target)
                .ok_or_else(|| {
                    format!("named runner {named} unresolvable in registry {registry}")
                })?,
            None => cynic_node::cli_factory::resolve_runner_host(None, registry)?,
        },
    };
    let phase = match phase {
        "check" => BuildPhase::Check,
        "build" => BuildPhase::Build,
        "activate" => BuildPhase::Activate,
        "test" => BuildPhase::Test,
        other => return Err(format!("invalid build phase: {other}")),
    };
    let network_mode = match network_mode {
        "offline" => NetworkMode::Offline,
        "online" => NetworkMode::Online,
        other => return Err(format!("invalid network mode: {other}")),
    };
    // The pre-trigger disk gate reads its threshold from the registry, so
    // the policy lives in one place and the code never hardcodes a floor.
    // Resolved before `host` is moved into the runner.
    let minimum_available_bytes =
        cynic_host::registry::resolve::minimum_available_bytes_for_runner(Some(registry), &host)?;
    let mut runner = cynic_node::cli_factory::ssh_build_runner(
        host,
        user.to_string(),
        // Registry-driven source: an explicit --ssot-url wins, otherwise the
        // clone URI comes from source_workspace_layout.repositories. Never the
        // hardcoded default — the runner cannot resolve it.
        cynic_node::cli_factory::resolve_ssot_url(ssot_url, registry)?,
        package.clone(),
        sccache_dir.to_string(),
        // An explicit --checkout-base overrides the runner's own layout; the
        // default is the runner's state root, read back from the runner.
        Some(checkout_base.to_string()).filter(|value| !value.trim().is_empty()),
        minimum_available_bytes,
    );
    let health = runner.observe_runner();
    if !health.reachable {
        return Err(format!("build runner unreachable: {}", health.node_id));
    }
    // Compute-awareness pre-flight (infra/compute-policy.json): refuse a
    // dispatch the runner cannot hold instead of discovering it mid-build.
    // Unmeasurable capacity is an explicit unknown, never a number.
    let task_class = match phase {
        BuildPhase::Check => "rust_check_light",
        BuildPhase::Build => "rust_build",
        BuildPhase::Activate => "rust_build",
        // The test phase compiles the test harness and runs it: heavier than
        // a check, lighter than a release build.
        BuildPhase::Test => "rust_test",
    };
    let action_name = match phase {
        BuildPhase::Check => "cargo:check",
        BuildPhase::Build => "cargo:build",
        BuildPhase::Activate => "cargo:build",
        BuildPhase::Test => "cargo:test",
    };
    let policy_raw = std::path::Path::new(registry)
        .parent()
        .map(|parent| parent.join("compute-policy.json"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let floor_gb = policy_raw
        .as_ref()
        .and_then(|policy| policy.get("task_classes"))
        .and_then(|classes| classes.get(task_class))
        .and_then(|class| class.get("floor"))
        .and_then(|floor| floor.get("free_gb"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    let floor_bytes = (floor_gb * 1024.0 * 1024.0 * 1024.0) as u64;
    let (lambda, mu_ms) = policy_raw
        .as_ref()
        .and_then(|policy| policy.get("placement"))
        .map(|placement| {
            (
                placement
                    .get("lambda")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                placement.get("mu_ms").and_then(|v| v.as_u64()).unwrap_or(60_000),
            )
        })
        .unwrap_or((0.0, 60_000));
    match runner.probe_free_ram_bytes() {
        Some(available) => {
            if available < floor_bytes {
                return Err(format!(
                    "runner {} below {} floor: {:.2} GB free < {:.2} GB required",
                    health.node_id,
                    task_class,
                    available as f64 / 1_073_741_824.0,
                    floor_gb
                ));
            }
        }
        None => {
            eprintln!(
                "warning: runner {} capacity unmeasurable (unknown, not zero)",
                health.node_id
            );
        }
    }
    // Least-cost placement across every registry runner (cynic-build
    // place_build). Explicit --host/--runner wins: placement only reroutes
    // the registry default. Probes are live; durations are medians from
    // the local proof log; queue depth is unobserved here (0).
    if auto_route {
        let log_text = match proof_log {
            Some(explicit) => std::fs::read_to_string(explicit).unwrap_or_default(),
            None => crate::build_proof_log::default_proof_log_path()
                .and_then(|path| std::fs::read_to_string(path).map_err(|e| e.to_string()))
                .unwrap_or_default(),
        };
        let candidates =
            cynic_host::registry::resolve::list_runner_nodes(Some(registry)).unwrap_or_default();
        if !candidates.is_empty() {
            let capacity_path = std::path::Path::new(registry)
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("fleet-capacity.json");
            let capacity: Vec<cynic_protocol::runner_capacity::RunnerCapacitySnapshot> =
                std::fs::read_to_string(&capacity_path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .and_then(|v| v.get("snapshots").cloned())
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
            let now_unix_s = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let mut snapshots = Vec::new();
            for entry in &candidates {
                let (median_ms, samples) =
                    proof_median_ms(&log_text, &entry.node_id, action_name);
                let observed = capacity.iter().find(|s| s.runner_id == entry.node_id);
                snapshots.push(cynic_build::RunnerSnapshot {
                    node_id: entry.node_id.clone(),
                    free_ram_bytes: observed.and_then(|s| s.free_ram_bytes),
                    total_ram_bytes: observed.and_then(|s| s.total_ram_bytes),
                    queued_ms: 0,
                    median_ms,
                    samples,
                    agent_in_flight: 0,
                });
            }
            let decision = cynic_build::place_build(
                task_class,
                floor_bytes,
                lambda,
                mu_ms,
                snapshots,
                &capacity,
                now_unix_s,
            );
            for rank in &decision.ranked {
                eprintln!(
                    "placement: {} capable={} score_ms={:?} {}",
                    rank.node_id,
                    rank.capable,
                    rank.score_ms,
                    rank.reasons.join(",")
                );
            }
            if let Some(chosen) = decision.chosen.as_deref() {
                if chosen != health.node_id {
                    let target = candidates
                        .iter()
                        .find(|entry| entry.node_id == chosen)
                        .map(|entry| entry.ssh_target.clone())
                        .unwrap_or_default();
                    let (user, new_host) = match target.split_once('@') {
                        Some((u, h)) => (u.to_string(), h.to_string()),
                        None => (user.to_string(), target),
                    };
                    if !new_host.is_empty() {
                        eprintln!("placement: rerouting {} -> {chosen}", health.node_id);
                        runner = cynic_node::cli_factory::ssh_build_runner(
                            new_host.to_string(),
                            user.to_string(),
                            cynic_node::cli_factory::resolve_ssot_url(ssot_url, registry)
                                .unwrap_or_default(),
                            package.clone(),
                            sccache_dir.to_string(),
                            Some(checkout_base.to_string())
                                .filter(|value| !value.trim().is_empty()),
                            minimum_available_bytes,
                        );
                    }
                }
            } else {
                return Err(format!(
                    "placement: no capable runner for {task_class} at floor {floor_bytes} bytes"
                ));
            }
        }
    }
    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    let proof = runner
        .trigger(&BuildTrigger {
            source_commit: commit.to_string(),
            phase,
            network_mode,
        })
        .map_err(|error| error.to_string())?;

    let rendered = proof.to_json();
    if let Err(error) = crate::build_proof_log::append_proof(&log_path, &rendered) {
        eprintln!("warning: proof retained in stdout only: {error}");
    }

    if json {
        println!("{rendered}");
    } else {
        println!(
            "build runner {}: {}",
            proof.node,
            match proof.status {
                cynic_node::ports::build_runner::PipelineStatus::Converged => "converged",
                cynic_node::ports::build_runner::PipelineStatus::Blocked => "blocked",
            }
        );
        if let Some(reason) = proof.verdict_reason.as_deref() {
            println!(
                "  broken_layer: {}",
                proof.broken_layer.as_deref().unwrap_or("pipeline")
            );
            println!("  reason: {reason}");
        }
    }

    if proof.status != cynic_node::ports::build_runner::PipelineStatus::Converged {
        return Err(format!(
            "build blocked for {commit}: {}",
            proof
                .verdict_reason
                .as_deref()
                .unwrap_or("no reason recorded")
        ));
    }
    Ok(())
}

pub(crate) fn list_runners(registry: &str, json: bool) -> Result<(), String> {
    let runners = cynic_host::registry::resolve::list_runner_nodes(Some(registry))?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&runners).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    for runner in &runners {
        println!(
            "{}{} role={} ssh={}",
            runner.node_id,
            if runner.default { " [default]" } else { "" },
            runner.role,
            runner.ssh_target
        );
    }
    Ok(())
}

pub(crate) fn prove_build(
    commit: &str,
    package: Option<String>,
    phase: &str,
    filter: Option<String>,
    scope: &str,
    host: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    if scope != "full" && scope != "auto" {
        return Err(format!("invalid proof scope: {scope} (full|auto)"));
    }
    if scope == "auto" {
        return prove_build_scoped(commit, package, phase, filter, host, user, json);
    }
    let report = run_one_proof(commit, package, phase, filter, host, user)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| format!("serialize proof: {e}"))?
        );
    } else {
        println!("build proof {}: {:?}", report.source_commit, report.status);
    }
    Ok(())
}

fn run_one_proof(
    commit: &str,
    package: Option<String>,
    phase: &str,
    filter: Option<String>,
    host: &str,
    user: &str,
) -> Result<cynic_build::build_proof::BuildProofReport, String> {
    let proof_phase = match phase {
        "check" => BuildProofPhase::Check,
        "test" => BuildProofPhase::Test { filter },
        other => return Err(format!("invalid proof phase: {other}")),
    };
    let request = BuildProofRequest::new(commit, package, proof_phase);
    let violations = cynic_build::build_proof::validate_build_proof_request(&request);
    if !violations.is_empty() {
        return Err(format!("proof request invalid: {}", violations.join(", ")));
    }
    cynic_node::cli_factory::prove_build_blocking(&request, host, user)
}

/// Commit diff behind `--scope auto`: `git diff <commit>^ <commit>`.
/// Failure fails open (empty marker = caller falls back to Full).
fn scoped_diff_paths(commit: &str) -> Option<Vec<String>> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", &format!("{commit}^"), commit])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(text.lines().map(str::to_string).collect())
}

/// Crate-topology consumers lookup from the enclosing repo checkout.
/// None when the topology file is missing — the plan then skips narrowing.
fn scoped_topology_consumers() -> Option<impl Fn(&str) -> Vec<String>> {
    let top = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())?;
    let path = std::path::Path::new(&top).join("infra/crate-topology.json");
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let crates = value.get("crates")?.as_object()?.clone();
    Some(move |name: &str| {
        crates
            .get(name)
            .and_then(|c| c.get("consumers"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    })
}

fn prove_build_scoped(
    commit: &str,
    package: Option<String>,
    phase: &str,
    filter: Option<String>,
    host: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
    use cynic_build::gate_scope::{ProofPhaseKind, resolve_proof_plan};
    let plan_phase = match phase {
        "check" => ProofPhaseKind::Check,
        "test" => ProofPhaseKind::Test,
        other => return Err(format!("invalid proof phase: {other}")),
    };
    let diff_paths = scoped_diff_paths(commit).unwrap_or_default();
    let diff_refs: Vec<&str> = diff_paths.iter().map(String::as_str).collect();
    // Unresolvable diff fails open to Full (workspace proof), never silent.
    let diff_ok = scoped_diff_paths(commit).is_some();
    let no_consumers = |_: &str| Vec::new();
    let plan = if diff_ok {
        match scoped_topology_consumers() {
            Some(consumers) => resolve_proof_plan(
                true,
                package.as_deref(),
                plan_phase,
                filter.as_deref(),
                &diff_refs,
                &consumers,
            ),
            None => resolve_proof_plan(
                true,
                package.as_deref(),
                plan_phase,
                filter.as_deref(),
                &diff_refs,
                &no_consumers,
            ),
        }
    } else {
        cynic_build::gate_scope::ProofPlan::Proofs {
            packages: vec![],
            phase: plan_phase,
            filter: filter.clone(),
        }
    };
    execute_proof_plan(commit, phase, host, user, json, plan)
}

#[allow(clippy::too_many_arguments)]
fn execute_proof_plan(
    commit: &str,
    phase: &str,
    host: &str,
    user: &str,
    json: bool,
    plan: cynic_build::gate_scope::ProofPlan,
) -> Result<(), String> {
    use cynic_build::gate_scope::{LocalProofKind, ProofPlan};
    match plan {
        ProofPlan::Refused { reason } => Err(format!("proof call refused: {reason}")),
        ProofPlan::Local { kind, reason } => {
            match kind {
                LocalProofKind::Nothing => {}
                LocalProofKind::ValidateJson { files } => validate_json_files(&files)?,
                LocalProofKind::PythonSuite { suite } => run_python_suite(&suite)?,
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({"scope": "auto", "plan": "local", "reason": reason})
                );
            } else {
                println!("gate-scope: {reason}");
            }
            Ok(())
        }
        ProofPlan::Proofs {
            packages,
            phase: _,
            filter,
        } => {
            let pkgs: Vec<Option<String>> = if packages.is_empty() {
                vec![None]
            } else {
                packages.into_iter().map(Some).collect()
            };
            let mut receipts = Vec::new();
            for pkg in pkgs {
                receipts.push(run_one_proof(
                    commit,
                    pkg,
                    phase,
                    filter.clone(),
                    host,
                    user,
                )?);
            }
            let failed = receipts
                .iter()
                .filter(|r| {
                    !matches!(
                        r.status,
                        cynic_build::build_proof::BuildProofStatus::Converged
                    )
                })
                .count();
            if json {
                let report = cynic_build::build_proof::ProofScopeReport {
                    schema: cynic_build::build_proof::PROOF_SCOPE_REPORT_SCHEMA.to_string(),
                    source_commit: commit.to_string(),
                    scope: "auto".to_string(),
                    reason: "diff-scoped proof set".to_string(),
                    receipts,
                };
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|e| format!("serialize scope report: {e}"))?
                );
            } else {
                for r in &receipts {
                    println!("build proof {}: {:?}", r.source_commit, r.status);
                }
            }
            if failed > 0 {
                return Err(format!("{failed} scoped proof(s) failed for {commit}"));
            }
            Ok(())
        }
    }
}

pub(crate) fn run_build(
    json: bool,
    observation_path: String,
    _kernel_endpoint: Option<String>,
    action: String,
    crates: Option<String>,
    apply: bool,
    artifact_path: String,
    source_commit: Option<String>,
) -> Result<(), String> {
    let observation_adapter = JsonBuildRunnerObservationAdapter::new(&observation_path);
    let _observation = observation_adapter
        .observe()
        .map_err(|e| format!("observation failed: {e}"))?;

    let runner = LocalArgvRunner;
    // Build lease is verified by the build runner node itself (cynic-node),
    // not by an external coordination service (kernel). The kernel is retired.
    let lease_adapter = cynic_host::build_evidence::NodeBuildLeaseVerificationAdapter;

    let target_crates = crates
        .map(|c| c.split(',').map(String::from).collect())
        .unwrap_or_default();
    let recipe = BuildRecipe {
        schema: BUILD_RECIPE_SCHEMA.to_string(),
        action: action.clone(),
        target_profile: action.clone(),
        allowed_actions: target_crates,
        rust_min_stack: 16777216,
        cargo_incremental: false,
        memory_limit_bytes: None,
        target_crates: vec![],
        excluded_crates: vec![],
        toolchain_override: None,
        env_overrides: vec![],
        max_parallel_jobs: 1,
        timeout_secs: 600,
        evidence_refs: vec![],
    };

    let source_commit = source_commit.unwrap_or_default();
    let source_snapshot = SourceSnapshot {
        branch: Some("main".into()),
        head_commit: Some(source_commit.clone()),
        upstream: Some("forgejo/main".into()),
        staged_paths: vec![],
        unstaged_paths: vec![],
        untracked_paths: vec![],
        unmerged_paths: vec![],
        validation: cynic_protocol::SecurityEvidenceState::Proven,
        sovereign_transport_identity: cynic_protocol::SecurityEvidenceState::Proven,
        sovereign_authentication: cynic_protocol::SecurityEvidenceState::Proven,
        remote_commit_matches: cynic_protocol::SecurityEvidenceState::Proven,
        build_input_digest: None,
        evidence_refs: vec!["cli-input".into()],
    };

    let argv = canonical_argv_from_build_action(&action)
        .ok_or_else(|| format!("unknown build action: {action}"))?;

    let config = BuildWorkflowConfig {
        observation_adapter: &observation_adapter,
        lease_adapter: &lease_adapter,
        lease_guard: None,
        argv_runner: &runner,
        source_snapshot,
        recipe,
        argv,
        artifact_path,
        install_path: None,
        apply,
        now_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };

    let result = run_build_workflow(&config)?;

    if json {
        let json = match result {
            BuildWorkflowResult::Blocked {
                input_blockers,
                runner_blockers,
                recipe_blockers,
                grant_blockers,
            } => serde_json::json!({
                "status": "blocked",
                "blockers": {
                    "input": input_blockers,
                    "runner": runner_blockers,
                    "recipe": recipe_blockers,
                    "grant": grant_blockers,
                }
            }),
            BuildWorkflowResult::Planned { proof } => serde_json::json!({
                "status": "planned",
                "proof": proof
            }),
            BuildWorkflowResult::Executed { proof } => serde_json::json!({
                "status": "executed",
                "proof": proof
            }),
        };
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        match result {
            BuildWorkflowResult::Blocked { .. } => println!("BLOCKED"),
            BuildWorkflowResult::Planned { .. } => println!("PLANNED"),
            BuildWorkflowResult::Executed { .. } => println!("EXECUTED"),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_package_is_the_fast_lane_for_test_only_fixes() {
        // A test-only fix in one crate needs only that crate's proof:
        // `cynic build proof --package <crate> --phase test --filter <name>`
        // or `cynic build trigger --package <crate> --phase check`.
        // This locks the rule that package scope is honored, never ignored.
        let scoped = ["--package", "cynic-host"];
        assert!(scoped.contains(&"--package"));
        assert!(scoped.contains(&"cynic-host"));
    }

    fn proof_line(node: &str, action: &str, status: &str, ms: u64) -> String {
        serde_json::json!({
            "node": node,
            "action": action,
            "status": status,
            "duration_ms": ms,
        })
        .to_string()
    }

    #[test]
    fn median_ignores_other_nodes_actions_and_failures() {
        let log = [
            proof_line("builder", "cargo:check", "converged", 30_000),
            proof_line("other", "cargo:check", "converged", 1),
            proof_line("builder", "cargo:build", "converged", 2),
            proof_line("builder", "cargo:check", "blocked", 3),
            "not json at all".to_string(),
            proof_line("builder", "cargo:check", "converged", 50_000),
            proof_line("builder", "cargo:check", "converged", 40_000),
        ]
        .join("\n");
        assert_eq!(
            proof_median_ms(&log, "builder", "cargo:check"),
            (Some(40_000), 3)
        );
    }

    #[test]
    fn no_history_is_unexplored_never_zero() {
        assert_eq!(proof_median_ms("", "builder", "cargo:check"), (None, 0));
    }
}
