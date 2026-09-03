//! Build CLI surface — thin facade over the build workflow.
//!
//! This is the CLI entry point for the build capability. It parses arguments,
//! injects the real adapters (SSH, HTTP, filesystem), and prints JSON.
//!
//! The CLI is orchestration only — no policy tables, no decision logic.

use clap::Subcommand;

use cynic_host::build_evidence::{BuildRunnerObservationPort, JsonBuildRunnerObservationAdapter};
use cynic_host::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_node::ports::build_runner::{BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode};
use cynic_node::ports::build_runner_adapter::{LocalCommandRunner, SshBuildRunner};
use cynic_protocol::build::{BUILD_RECIPE_SCHEMA, BuildRecipe, canonical_argv_from_build_action};

use crate::workload_observation::{execute_list_workloads, execute_observe_workload};

/// CLI arguments for the build surface.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildCommand {
    /// Run the build workflow (observe → verify → decide → execute)
    Run {
        /// Print JSON output
        #[arg(long)]
        json: bool,
        /// Path to the build runner observation JSON file
        #[arg(long, default_value = "/tmp/cynic-build-runner-observation.json")]
        observation_path: String,
        /// Kernel coordination endpoint for lease verification
        #[arg(long)]
        kernel_endpoint: Option<String>,
        /// Build action (make:check, make:build, make:build-release)
        #[arg(long, default_value = "make:check")]
        action: String,
        /// Target crates (comma-separated)
        #[arg(long)]
        crates: Option<String>,
        /// Apply the build (execute if admitted)
        #[arg(long)]
        apply: bool,
        /// Explicit lease guard JSON produced by the claim/lease path.
        #[arg(long)]
        lease_guard: Option<String>,
        /// Artifact path for read-back proof
        #[arg(long, default_value = "target/release/cynic")]
        artifact_path: String,
        /// Observed source snapshot JSON proof. Without it, source trust remains unknown and execution blocks.
        #[arg(long)]
        source_snapshot: Option<String>,
        /// Source commit hash (must match the observed snapshot when both are supplied)
        #[arg(long)]
        source_commit: Option<String>,
    },
    /// Trigger a distributed build on the observed network runner.
    Trigger {
        /// Exact 40-character source commit.
        #[arg(long)]
        commit: String,
        /// Build phase: check, build, or activate.
        #[arg(long, default_value = "check")]
        phase: String,
        /// Runner host resolved from the fleet registry.
        #[arg(long, default_value = "10.10.10.119")]
        host: String,
        /// Runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Network mode: offline or online.
        #[arg(long, default_value = "offline")]
        network_mode: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Observe one workload by id across the federation (local store + SSH).
    ObserveWorkload {
        /// Workload id to observe.
        #[arg(long)]
        workload_id: String,
        /// Node state root on remote nodes.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Restrict to the local store (no SSH federation).
        #[arg(long)]
        local: bool,
        /// Path to the registry JSON for node discovery.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// SSH user for remote nodes.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List all durable workloads across the federation (local store + SSH).
    ListWorkloads {
        /// Node state root on remote nodes.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Restrict to the local store (no SSH federation).
        #[arg(long)]
        local: bool,
        /// Path to the registry JSON for node discovery.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// SSH user for remote nodes.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Serve HTTP webhook receiver for Forgejo push events.
    ServeWebhook {
        /// Address to bind (e.g. 127.0.0.1:9147).
        #[arg(long, default_value = "127.0.0.1:9147")]
        bind: String,
        /// Build phase to trigger on push (check|build|activate).
        #[arg(long, default_value = "check")]
        phase: String,
        /// Path to the registry JSON for node discovery.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Kernel coordination endpoint for lease verification.
        #[arg(long)]
        kernel_endpoint: Option<String>,
        /// SSH target for the build runner (e.g. cynic@10.10.10.119).
        #[arg(long, default_value = "cynic@10.10.10.119")]
        runner: String,
        /// Apply builds after webhook admission. Without this flag, only a plan is produced.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Provision SSH credentials for all machine_enrollment transports in the registry.
    ProvisionCredentials {
        /// Path to the registry JSON.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Secrets directory (default: /etc/cynic/secrets).
        #[arg(long, default_value = "/etc/cynic/secrets")]
        secrets_dir: String,
        /// Apply provisioning (generate keys). Without this, only scan.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Discover workspace crates/toolchain/targets (thin: Local/Remote adapter -> JSON).
    Discover {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Discovery kind: local (default) or remote (requires --target).
        #[arg(long, default_value = "local", value_parser = ["local","remote"])]
        kind: String,
        /// SSH target for remote kind (e.g. cynic@10.0.0.1).
        #[arg(long)]
        target: Option<String>,
    },
}

/// Execute the build CLI command.
pub(crate) fn execute_build(command: BuildCommand) -> Result<(), String> {
    match command {
        BuildCommand::Run {
            json,
            observation_path,
            kernel_endpoint,
            action,
            crates,
            apply,
            lease_guard,
            artifact_path,
            source_snapshot,
            source_commit,
        } => run_build(
            json,
            observation_path,
            kernel_endpoint,
            action,
            crates,
            apply,
            lease_guard,
            artifact_path,
            source_snapshot,
            source_commit,
        ),
        BuildCommand::Trigger {
            commit,
            phase,
            host,
            user,
            network_mode,
            json,
        } => trigger_build(&commit, &phase, &host, &user, &network_mode, json),
        BuildCommand::ObserveWorkload {
            workload_id,
            state_root,
            local,
            registry,
            user,
            json,
        } => execute_observe_workload(&workload_id, &state_root, local, &registry, &user, json),
        BuildCommand::ListWorkloads {
            state_root,
            local,
            registry,
            user,
            json,
        } => execute_list_workloads(&state_root, local, &registry, &user, json),
        BuildCommand::ServeWebhook {
            bind,
            phase,
            registry,
            kernel_endpoint,
            runner,
            apply,
            json,
        } => serve_webhook(
            &bind,
            &phase,
            &registry,
            kernel_endpoint,
            &runner,
            apply,
            json,
        ),
        BuildCommand::ProvisionCredentials {
            registry,
            secrets_dir,
            apply,
            json,
        } => provision_credentials(&registry, &secrets_dir, apply, json),
        BuildCommand::Discover { json, kind, target } => {
            let code = crate::build_discovery_surface::execute_build_discover(
                json,
                &kind,
                target.as_deref(),
            );
            return match code {
                c if c == std::process::ExitCode::SUCCESS => Ok(()),
                _ => Err("build discover failed".to_string()),
            };
        }
    }
}

fn trigger_build(
    commit: &str,
    phase: &str,
    host: &str,
    user: &str,
    network_mode: &str,
    json: bool,
) -> Result<(), String> {
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
    let runner = SshBuildRunner {
        host: host.to_string(),
        user: user.to_string(),
        runner: Box::new(LocalCommandRunner),
    };
    let health = runner.observe_runner();
    if !health.reachable {
        return Err(format!("build runner unreachable: {}", health.node_id));
    }
    let proof = runner
        .trigger(&BuildTrigger {
            source_commit: commit.to_string(),
            phase,
            network_mode,
        })
        .map_err(|error| error.to_string())?;
    if json {
        println!("{}", proof.to_json());
    } else {
        println!("build runner {}: {:?}", proof.node, proof.status);
    }
    Ok(())
}

fn run_build(
    json: bool,
    observation_path: String,
    kernel_endpoint: Option<String>,
    action: String,
    crates: Option<String>,
    apply: bool,
    lease_guard_path: Option<String>,
    artifact_path: String,
    source_snapshot_path: Option<String>,
    source_commit: Option<String>,
) -> Result<(), String> {
    let observation_adapter = JsonBuildRunnerObservationAdapter::new(&observation_path);
    let _observation = observation_adapter
        .observe()
        .map_err(|e| format!("observation failed: {e}"))?;

    // Step 2: Create lease verification adapter
    let runner = LocalArgvRunner;
    let lease_adapter = if let Some(endpoint) = kernel_endpoint {
        cynic_host::build_evidence::CurlBuildLeaseVerificationAdapter::new(&runner, endpoint, None)
            .map_err(|e| format!("lease adapter failed: {e}"))?
    } else {
        return Err("kernel endpoint required for lease verification".to_string());
    };

    // Step 3: Build recipe
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

    // Source trust must come from an observed, serialized snapshot. A locally
    // fabricated `Proven` snapshot would turn missing provenance into authority.
    let source_snapshot = match source_snapshot_path {
        Some(path) => std::fs::read_to_string(&path)
            .map_err(|error| format!("read source snapshot {path}: {error}"))
            .and_then(|raw| {
                serde_json::from_str::<cynic_protocol::SourceSnapshot>(&raw)
                    .map_err(|error| format!("decode source snapshot {path}: {error}"))
            })?,
        None => {
            return Err("observed source snapshot required (--source-snapshot)".to_string());
        }
    };
    if let Some(expected_commit) = source_commit.as_deref()
        && source_snapshot.head_commit.as_deref() != Some(expected_commit)
    {
        return Err("source commit does not match observed source snapshot".to_string());
    }

    // Step 5: Run workflow — translate --action to argv via the canonical translator.
    // No silent fallback: an unknown action is a operator error, not a default.
    let argv = canonical_argv_from_build_action(&action)
        .ok_or_else(|| format!("unknown build action: {action}"))?;

    let lease_guard = match lease_guard_path {
        Some(path) => Some(
            std::fs::read_to_string(&path)
                .map_err(|error| format!("read lease guard {path}: {error}"))
                .and_then(|raw| {
                    serde_json::from_str::<cynic_protocol::LeaseGuard>(&raw)
                        .map_err(|error| format!("decode lease guard {path}: {error}"))
                })?,
        ),
        None => None,
    };

    let config = BuildWorkflowConfig {
        observation_adapter: &observation_adapter,
        lease_adapter: &lease_adapter,
        lease_guard,
        argv_runner: &runner,
        source_snapshot,
        recipe,
        argv,
        artifact_path,
        apply,
        now_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
    };

    let result = run_build_workflow(&config)?;

    // Step 6: Output
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

fn serve_webhook(
    bind: &str,
    _phase: &str,
    _registry: &str,
    _kernel_endpoint: Option<String>,
    runner: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    use cynic_host::forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};

    let config = WebhookServerConfig {
        bind: bind.to_string(),
        phase: _phase.to_string(),
        runner_target: runner.to_string(),
        apply,
        registry_path: _registry.to_string(),
    };

    let handle = spawn_webhook_server(config)
        .map_err(|e| format!("failed to bind webhook server on {bind}: {e}"))?;

    // Print startup message
    let startup = serde_json::json!({
        "status": "listening",
        "bind": bind,
        "runner_target": runner,
        "phase": _phase,
        "apply": apply,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&startup).unwrap());
    } else {
        println!("Forgejo webhook server listening on {bind}");
        println!("  Runner: {runner}");
        println!("  Phase: {_phase}");
        println!("  Autonomous apply: enabled");
    }

    // Block on server thread (it handles all incoming requests in-process)
    let _ = handle.join();
    Ok(())
}

fn provision_credentials(
    registry_path: &str,
    secrets_dir: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    use cynic_host::credential_provisioning::{
        provision_ssh_credential, scan_unprovisioned_enrollment_transports,
    };

    let registry_content = std::fs::read_to_string(registry_path)
        .map_err(|e| format!("failed to read registry: {e}"))?;
    let registry: serde_json::Value = serde_json::from_str(&registry_content)
        .map_err(|e| format!("failed to parse registry: {e}"))?;

    let secrets_path = std::path::Path::new(secrets_dir);
    let transports = scan_unprovisioned_enrollment_transports(&registry, secrets_path)?;

    if !apply {
        // Scan only — report what needs provisioning
        let report = serde_json::json!({
            "status": "scan_complete",
            "secrets_dir": secrets_dir,
            "nodes_needing_credentials": transports.iter().map(|t| &t.node_id).collect::<Vec<_>>(),
            "count": transports.len(),
            "apply_requested": false,
        });
        if json {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!("SSH credentials scan:");
            if transports.is_empty() {
                println!("  All transports already provisioned");
            } else {
                for t in &transports {
                    println!(
                        "  - {} ({}) needs SSH key: {}",
                        t.node_id, t.ssh_target, t.credential_ref
                    );
                }
            }
        }
        return Ok(());
    }

    // Apply provisioning — generate SSH keys for each unprovisioned transport
    let runner = LocalArgvRunner;
    let mut proofs = Vec::new();
    let mut errors = Vec::new();

    for transport in &transports {
        match provision_ssh_credential(transport, secrets_path, &runner) {
            Ok(proof) => proofs.push(serde_json::json!({
                "node_id": proof.node_id,
                "ssh_key_path": proof.ssh_key_path,
                "public_key_sha256": proof.public_key_sha256,
                "known_hosts_configured": proof.known_hosts_configured,
                "read_back_verified": proof.read_back_verified,
            })),
            Err(e) => errors.push(serde_json::json!({
                "node_id": transport.node_id,
                "error": e,
            })),
        }
    }

    let report = serde_json::json!({
        "status": if errors.is_empty() { "provisioned" } else { "partial_failure" },
        "secrets_dir": secrets_dir,
        "provisioned": proofs,
        "errors": errors,
        "total": transports.len(),
        "succeeded": proofs.len(),
        "failed": errors.len(),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("SSH credential provisioning:");
        for p in &proofs {
            println!("  + {} → {}", p["node_id"], p["ssh_key_path"]);
        }
        for e in &errors {
            println!("  ✗ {} → {}", e["node_id"], e["error"]);
        }
        println!("  {} provisioned, {} failed", proofs.len(), errors.len());
    }

    if !errors.is_empty() {
        return Err(format!("provisioning failed for {} nodes", errors.len()));
    }

    Ok(())
}
