//! Build triggering, execution and proof verification.

use cynic_build::build::{BUILD_RECIPE_SCHEMA, BuildRecipe, canonical_argv_from_build_action};
use cynic_build::build_proof::{BuildProofPhase, BuildProofRequest};
use cynic_host::build_evidence::{BuildRunnerObservationPort, JsonBuildRunnerObservationAdapter};
use cynic_host::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};
use cynic_lifecycle::LocalArgvRunner;
use cynic_node::ports::build_runner::{BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode};
use cynic_vcs::SourceSnapshot;

pub(crate) fn resolve_default_builder_url() -> Result<String, String> {
    cynic_build::resolve_builder_url()
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
    if let Some(warning) = package_scope_warning(action, package) {
        eprintln!("warning: {warning}");
    }
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
    Ok(())
}

/// Pure predicate: `--package` currently reaches the builder payload but no
/// downstream composition reads it, so every action runs workspace-wide.
/// Returns the operator warning, or `None` when there is nothing to warn.
pub(crate) fn package_scope_warning(action: &str, package: Option<&str>) -> Option<String> {
    package.map(|pkg| {
        format!(
            "--package {pkg} is currently ignored: action `{action}` runs workspace-wide on the builder"
        )
    })
}

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
    sccache_dir: &str,
    proof_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let host = match host.filter(|value| !value.trim().is_empty()) {
        Some(explicit) => explicit.to_string(),
        None => match runner.filter(|value| !value.trim().is_empty()) {
            Some(named) => {
                cynic_host::registry::resolve::list_runner_nodes(Some(registry))?
                    .into_iter()
                    .find(|candidate| candidate.node_id == named)
                    .map(|candidate| candidate.ssh_target)
                    .ok_or_else(|| {
                        format!("named runner {named} unresolvable in registry {registry}")
                    })?
            }
            None => cynic_node::cli_factory::resolve_runner_host(None, registry)?,
        },
    };
    let phase = match phase {
        "check" => BuildPhase::Check,
        "build" => BuildPhase::Build,
        "activate" => BuildPhase::Activate,
        other => return Err(format!("invalid build phase: {other}")),
    };
    let network_mode = match network_mode {
        "offline" => NetworkMode::Offline,
        "online" => NetworkMode::Online,
        other => return Err(format!("invalid network mode: {other}")),
    };
    let mut runner = cynic_node::cli_factory::ssh_build_runner(
        host,
        user.to_string(),
        checkout_base.to_string(),
        // Registry-driven source: an explicit --ssot-url wins, otherwise the
        // clone URI comes from source_workspace_layout.repositories. Never the
        // hardcoded default — the runner cannot resolve it.
        cynic_node::cli_factory::resolve_ssot_url(ssot_url, registry)?,
        package,
        sccache_dir.to_string(),
    );
    let health = runner.observe_runner();
    if !health.reachable {
        return Err(format!("build runner unreachable: {}", health.node_id));
    }
    // sccache auto-enable: the wrapper is content-keyed (hits byte-identical
    // across SHAs), so a runner that carries the binary gets it by default.
    // Absence falls back to unwrapped, explicitly, never silently.
    if runner.sccache_dir.is_none() {
        if runner.probe_sccache_available() {
            runner.sccache_dir = Some(
                cynic_node::ports::build_runner_adapter::default_sccache_dir()
                    .display()
                    .to_string(),
            );
        } else {
            eprintln!(
                "warning: runner {} has no sccache wrapper, building unwrapped",
                health.node_id
            );
        }
    }
    // Compute-awareness pre-flight (infra/compute-policy.json): refuse a
    // dispatch the runner cannot hold instead of discovering it mid-build.
    // Unmeasurable capacity is an explicit unknown, never a number.
    let task_class = match phase {
        BuildPhase::Check => "rust_check_light",
        BuildPhase::Build => "rust_build",
        BuildPhase::Activate => "rust_build",
    };
    let action_name = match phase {
        BuildPhase::Check => "cargo:check",
        BuildPhase::Build => "cargo:build",
        BuildPhase::Activate => "cargo:build",
    };
    let policy_raw = std::path::Path::new(registry)
        .parent()
        .map(|parent| parent.join("compute-policy.json"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
    let floor_bytes = policy_raw
        .as_ref()
        .and_then(|policy| policy.get("task_classes"))
        .and_then(|classes| classes.get(task_class))
        .and_then(|class| class.get("floor"))
        .and_then(|floor| floor.get("free_gb"))
        .and_then(|value| value.as_f64())
        .map(|floor_gb| (floor_gb * 1024.0 * 1024.0 * 1024.0) as u64)
        .unwrap_or(0);
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
                    floor_bytes as f64 / 1_073_741_824.0
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
    // place_build). Probes are live; durations are medians from the local
    // proof log; queue depth is unobserved from the trigger path (0 with
    // the admission queue pending scheduler-side).
    let log_text = match proof_log {
        Some(explicit) => std::fs::read_to_string(explicit).unwrap_or_default(),
        None => crate::build_proof_log::default_proof_log_path()
            .and_then(|path| std::fs::read_to_string(path).map_err(|e| e.to_string()))
            .unwrap_or_default(),
    };
    let candidates =
        cynic_host::registry::resolve::list_runner_nodes(Some(registry)).unwrap_or_default();
    if !candidates.is_empty() {
        let mut snapshots = Vec::new();
        for entry in &candidates {
            let Some((user, host)) = entry.ssh_target.split_once('@') else {
                continue;
            };
            let probe = cynic_node::cli_factory::ssh_build_runner(
                host.to_string(),
                user.to_string(),
                checkout_base.to_string(),
                cynic_node::cli_factory::resolve_ssot_url(ssot_url, registry)
                    .unwrap_or_default(),
                package.clone(),
                sccache_dir.to_string(),
            );
            let (median_ms, samples) =
                proof_median_ms(&log_text, &entry.node_id, action_name);
            snapshots.push(cynic_build::RunnerSnapshot {
                node_id: entry.node_id.clone(),
                free_ram_bytes: probe.probe_free_ram_bytes(),
                total_ram_bytes: probe.probe_total_ram_bytes(),
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
                if let Some((user, host)) = target.split_once('@') {
                    eprintln!("placement: rerouting {} -> {chosen}", health.node_id);
                    runner = cynic_node::cli_factory::ssh_build_runner(
                        host.to_string(),
                        user.to_string(),
                        checkout_base.to_string(),
                        cynic_node::cli_factory::resolve_ssot_url(ssot_url, registry)
                            .unwrap_or_default(),
                        package.clone(),
                        sccache_dir.to_string(),
                    );
                }
            }
        } else {
            return Err(format!(
                "placement: no capable runner for {task_class} at floor {} bytes",
                floor_bytes
            ));
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
    host: &str,
    user: &str,
    json: bool,
) -> Result<(), String> {
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
    let report = cynic_node::cli_factory::prove_build_blocking(&request, host, user)?;
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

pub(crate) fn run_build(
    json: bool,
    observation_path: String,
    kernel_endpoint: Option<String>,
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
    let lease_adapter = if let Some(endpoint) = kernel_endpoint {
        cynic_host::build_evidence::CurlBuildLeaseVerificationAdapter::new(&runner, endpoint, None)
            .map_err(|e| format!("lease adapter failed: {e}"))?
    } else {
        return Err("kernel endpoint required for lease verification".to_string());
    };

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
    fn package_warns_for_every_action_while_payload_is_dropped() {
        for action in ["check", "build", "build-release", "test"] {
            let warning =
                package_scope_warning(action, Some("cynic-cli")).expect("must warn");
            assert!(warning.contains("cynic-cli"), "{action}: {warning}");
            assert!(warning.contains("ignored"), "{action}: {warning}");
        }
    }

    #[test]
    fn no_package_means_no_warning() {
        assert_eq!(package_scope_warning("check", None), None);

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
