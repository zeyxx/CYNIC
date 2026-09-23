//! Build CLI surface — thin facade over the build workflow.
//!
//! This is the CLI entry point for the build capability. It parses arguments,
//! injects the real adapters (SSH, HTTP, filesystem), and prints JSON.
//!
//! The CLI is orchestration only — no policy tables, no decision logic.

use clap::Subcommand;

use cynic_build::build::{BUILD_RECIPE_SCHEMA, BuildRecipe, canonical_argv_from_build_action};
use cynic_build::build_proof::{BuildProofPhase, BuildProofRequest};
use cynic_host::build_evidence::{BuildRunnerObservationPort, JsonBuildRunnerObservationAdapter};
use cynic_host::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};
use cynic_lifecycle::LocalArgvRunner;
use cynic_node::adapters::ssh_build_proof::SshBuildProof;
use cynic_node::ports::build_proof_port::BuildProofPort;
use cynic_node::ports::build_runner::{BuildPhase, BuildRunnerPort, BuildTrigger, NetworkMode};
use cynic_node::ports::build_runner_adapter::{LocalCommandRunner, SshBuildRunner};
use cynic_vcs::SourceSnapshot;

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
        /// Artifact path for read-back proof
        #[arg(long, default_value = "target/release/cynic")]
        artifact_path: String,
        /// Source commit hash
        #[arg(long)]
        source_commit: Option<String>,
    },
    /// Submit a build workload natively to the builder HTTP node (no SSH).
    Submit {
        /// Exact 40-character source commit.
        #[arg(long)]
        commit: String,
        /// Optional single package scope. None = workspace.
        #[arg(long)]
        package: Option<String>,
        /// Action to execute: check, test, build, build-release.
        #[arg(long, default_value = "check")]
        action: String,
        /// Agent or requester identifier.
        #[arg(long, default_value = "cynic-operator")]
        requested_by: String,
        /// Builder HTTP endpoint (resolved dynamically from registry if omitted).
        #[arg(long)]
        builder_url: Option<String>,
        /// Wait for completion and fetch proof.
        #[arg(long)]
        wait: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Trigger a distributed build on the observed network runner.
    Trigger {
        /// Exact 40-character source commit.
        #[arg(long)]
        commit: String,
        /// Build phase: check, build, or activate.
        #[arg(long, default_value = "check")]
        phase: String,
        /// Explicit runner host override (breakglass only). When absent, the
        /// runner SSH target is resolved from the fleet registry.
        #[arg(long)]
        host: Option<String>,
        /// Path to the fleet registry JSON used for runner resolution.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Network mode: offline or online.
        #[arg(long, default_value = "offline")]
        network_mode: String,
        /// Checkout base dir on the runner for per-SHA checkouts.
        #[arg(long, default_value = "")]
        checkout_base: String,
        /// Source of truth cloned when the runner has no checkout.
        #[arg(long, default_value = "")]
        ssot_url: String,
        /// Optional single package scope. None = workspace.
        #[arg(long)]
        package: Option<String>,
        /// Shared sccache dir on the runner (content-keyed compiler cache).
        /// Empty = no wrapper (current behavior).
        #[arg(long, default_value = "")]
        sccache_dir: String,
        /// Proof log path. Defaults to the operator log under $HOME.
        #[arg(long)]
        proof_log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List retained trigger proofs from the operator proof log.
    ListProofs {
        /// Keep only proofs for this exact source commit.
        #[arg(long)]
        commit: Option<String>,
        /// Keep at most N newest proofs (0 = no limit).
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Proof log path. Defaults to the operator log under $HOME.
        #[arg(long)]
        proof_log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Aggregate pipeline proofs for one commit into a metabolic invoice.
    Invoice {
        /// Exact 40-character source commit.
        #[arg(long)]
        commit: String,
        /// Proof log path. Defaults to the operator log under $HOME.
        #[arg(long)]
        proof_log: Option<String>,
        /// Invoice log path. Defaults to the operator log under $HOME.
        #[arg(long)]
        invoice_log: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Prove a commit builds — native CYNIC proof capability.
    ///
    /// Replaces ad-hoc SSH+shell sequences with a typed, versioned proof.
    Proof {
        /// Exact 40-character source commit.
        #[arg(long)]
        commit: String,
        /// Optional single package scope. None = workspace.
        #[arg(long)]
        package: Option<String>,
        /// Proof phase: check (default) or test.
        #[arg(long, default_value = "check", value_parser = ["check", "test"])]
        phase: String,
        /// Focused test filter (only with --phase test).
        #[arg(long)]
        filter: Option<String>,
        /// Runner host resolved from the fleet registry.
        #[arg(long, default_value = "cynic-builder-ssh")]
        host: String,
        /// Runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
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
        /// Node builder webhook URL. Required: an empty endpoint is a blocked
        /// forwarding configuration, never a guessed runner or registry path.
        #[arg(long)]
        node_webhook_url: String,
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
            artifact_path,
            source_commit,
        } => run_build(
            json,
            observation_path,
            kernel_endpoint,
            action,
            crates,
            apply,
            artifact_path,
            source_commit,
        ),
        BuildCommand::Submit {
            commit,
            package,
            action,
            requested_by,
            builder_url,
            wait,
            json,
        } => {
            let url = match builder_url {
                Some(explicit) => explicit,
                None => resolve_default_builder_url()?,
            };
            submit_build(
                &commit,
                package.as_deref(),
                &action,
                &requested_by,
                &url,
                wait,
                json,
            )
        }
        BuildCommand::Trigger {
            commit,
            phase,
            host,
            registry,
            user,
            network_mode,
            checkout_base,
            ssot_url,
            package,
            sccache_dir,
            proof_log,
            json,
        } => trigger_build(
            &commit,
            &phase,
            host.as_deref(),
            &registry,
            &user,
            &network_mode,
            &checkout_base,
            &ssot_url,
            package,
            &sccache_dir,
            proof_log.as_deref(),
            json,
        ),
        BuildCommand::ListProofs {
            commit,
            limit,
            proof_log,
            json,
        } => list_proofs(commit.as_deref(), limit, proof_log.as_deref(), json),
        BuildCommand::Invoice {
            commit,
            proof_log,
            invoice_log,
            json,
        } => invoice_for_commit(&commit, proof_log.as_deref(), invoice_log.as_deref(), json),
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
            node_webhook_url,
            json,
        } => serve_webhook(&bind, &node_webhook_url, json),
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
        BuildCommand::Proof {
            commit,
            package,
            phase,
            filter,
            host,
            user,
            json,
        } => prove_build(&commit, package, &phase, filter, &host, &user, json),
    }
}

fn trigger_build(
    commit: &str,
    phase: &str,
    host: Option<&str>,
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
        None => {
            cynic_host::registry::resolve::resolve_build_runner_transport(Some(registry))
                .map_err(|error| {
                    format!("build runner unresolvable in registry {registry}: {error}")
                })?
                .ssh_target
        }
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
    let runner = SshBuildRunner {
        host: host.to_string(),
        user: user.to_string(),
        checkout_base: if checkout_base.is_empty() {
            cynic_node::ports::build_runner_adapter::DEFAULT_CHECKOUT_BASE.to_string()
        } else {
            checkout_base.to_string()
        },
        ssot_url: if ssot_url.is_empty() {
            cynic_node::ports::build_runner_adapter::DEFAULT_SSOT_URL.to_string()
        } else {
            ssot_url.to_string()
        },
        package,
        sccache_dir: if sccache_dir.trim().is_empty() {
            None
        } else {
            Some(sccache_dir.to_string())
        },
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
    let rendered = proof.to_json();
    if json {
        println!("{rendered}");
    } else {
        println!("build runner {}: {:?}", proof.node, proof.status);
    }
    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    if let Err(error) = crate::build_proof_log::append_proof(&log_path, &rendered) {
        eprintln!("warning: proof retained in stdout only: {error}");
    }
    Ok(())
}

fn list_proofs(
    commit: Option<&str>,
    limit: usize,
    proof_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    let (proofs, skipped) = crate::build_proof_log::read_proofs(&log_path, commit, limit)?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "proof_log": log_path, "proofs": proofs, "skipped_lines": skipped })
        );
    } else {
        println!("proof log {log_path}: {} retained", proofs.len());
        for proof in &proofs {
            println!("  {}", proof);
        }
        for skipped in &skipped {
            println!("  skipped: {skipped}");
        }
    }
    Ok(())
}

fn invoice_for_commit(
    commit: &str,
    proof_log: Option<&str>,
    invoice_log: Option<&str>,
    json: bool,
) -> Result<(), String> {
    use cynic_node::ports::build_runner::{BuildProofBundle, invoice_from_proofs};

    let log_path = match proof_log {
        Some(explicit) => explicit.to_string(),
        None => crate::build_proof_log::default_proof_log_path()?,
    };
    let (proofs, skipped) = crate::build_proof_log::read_proofs(&log_path, Some(commit), 0)?;
    let mut bundles = Vec::new();
    let mut legacy = 0usize;
    for proof in &proofs {
        match serde_json::from_value::<BuildProofBundle>(proof.clone()) {
            Ok(bundle) => bundles.push(bundle),
            Err(_) => legacy += 1,
        }
    }
    let mut invoice = invoice_from_proofs(commit, &bundles);
    // Observed proofs are the log lines; only parsed ones are invoiced.
    invoice.proofs_observed = proofs.len();
    if proofs.is_empty() {
        invoice
            .residual_unknowns
            .push("no proofs observed for commit".to_string());
    }
    for line in &skipped {
        invoice.residual_unknowns.push(line.clone());
    }
    if legacy > 0 {
        invoice.residual_unknowns.push(format!(
            "{legacy} proof(s) predate duration_ms and carry no observed wall time"
        ));
    }
    invoice
        .validate()
        .map_err(|blockers| format!("invoice invalid: {}", blockers.join(", ")))?;
    let rendered =
        serde_json::to_string_pretty(&invoice).map_err(|e| format!("serialize invoice: {e}"))?;
    if json {
        println!("{rendered}");
    } else {
        println!(
            "metabolic invoice {commit}: {} proofs invoiced, {} ms wall",
            invoice.proofs_invoiced, invoice.total.wall_time_ms
        );
        for unknown in &invoice.residual_unknowns {
            println!("  unknown: {unknown}");
        }
    }
    let journal_path = match invoice_log {
        Some(explicit) => explicit.to_string(),
        None => crate::invoice_log::default_invoice_log_path()?,
    };
    if let Err(error) = crate::invoice_log::append_invoice(&journal_path, &rendered) {
        eprintln!("warning: invoice retained in stdout only: {error}");
    }
    Ok(())
}

fn prove_build(
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
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("async runtime unavailable: {e}"))?;
    let adapter = SshBuildProof::new(host, user, "builder");
    let report = runtime.block_on(adapter.prove(&request))?;
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

fn resolve_default_builder_url() -> Result<String, String> {
    if let Ok(env_url) = std::env::var("CYNIC_BUILDER_URL") {
        if !env_url.trim().is_empty() {
            return Ok(env_url);
        }
    }
    let registry_path = std::path::Path::new("infra/registry.json");
    if registry_path.exists() {
        let content = std::fs::read_to_string(registry_path)
            .map_err(|e| format!("cannot read infra/registry.json: {e}"))?;
        let reg: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("invalid infra/registry.json format: {e}"))?;
        if let Some(nodes) = reg["nodes"].as_array() {
            for node in nodes {
                let role = node["role"].as_str().unwrap_or_default();
                if role == "build_runner" {
                    if let Some(ip) = node["network"]["synapse"].as_str() {
                        return Ok(format!("http://{ip}:9090"));
                    }
                }
            }
        }
    }
    Err("could not resolve builder URL dynamically from infra/registry.json or CYNIC_BUILDER_URL env".into())
}

fn submit_build(
    commit: &str,
    package: Option<&str>,
    action: &str,
    requested_by: &str,
    builder_url: &str,
    wait: bool,
    json: bool,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "commit_sha": commit,
        "package": package,
        "action": action,
        "requested_by": requested_by,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http client init failed: {e}"))?;

    let submit_url = format!("{}/workloads/submit", builder_url.trim_end_matches('/'));
    let res = client
        .post(&submit_url)
        .json(&payload)
        .send()
        .map_err(|e| format!("failed to submit workload to {submit_url}: {e}"))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().unwrap_or_default();
        return Err(format!("builder rejected submission ({status}): {body}"));
    }

    let proof: serde_json::Value = res
        .json()
        .map_err(|e| format!("failed to parse acceptance proof: {e}"))?;

    if !wait {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&proof).unwrap_or_default()
            );
        } else {
            let wid = proof["workload_id"].as_str().unwrap_or("unknown");
            println!("Workload admitted successfully: id={wid} (status: accepted)");
        }
        return Ok(());
    }

    let wid = proof["workload_id"].as_str().unwrap_or("");
    let proof_url = format!("{}/proofs/{}", builder_url.trim_end_matches('/'), wid);
    if !json {
        println!("Admitted workload {wid} to builder {builder_url}");
        println!("Streaming state updates (polling {})...", proof_url);
    }

    let start = std::time::Instant::now();
    let poll_timeout = std::time::Duration::from_secs(900);
    let mut last_state = String::new();

    while start.elapsed() < poll_timeout {
        std::thread::sleep(std::time::Duration::from_secs(3));
        if let Ok(pres) = client.get(&proof_url).send() {
            if pres.status().is_success() {
                if let Ok(pjson) = pres.json::<serde_json::Value>() {
                    let state = pjson["state"].as_str().unwrap_or("unknown");
                    if state != last_state {
                        if !json {
                            let elapsed = start.elapsed().as_secs();
                            println!(
                                "[+{elapsed}s] Workload {wid} state transition: {last_state} -> {state}"
                            );
                        }
                        last_state = state.to_string();
                    }

                    if state == "completed" || state == "failed" {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&pjson).unwrap_or_default()
                            );
                        } else {
                            println!(
                                "Build finished in {}s: state={state} id={wid}",
                                start.elapsed().as_secs()
                            );
                            if let Some(receipt) = pjson.get("parcel_receipt") {
                                if let Some(digest) =
                                    receipt.get("final_record_digest").and_then(|d| d.as_str())
                                {
                                    println!("Hermetic Parcel Digest: {digest}");
                                }
                                if let Some(settled_at) =
                                    receipt.get("settled_at_ms").and_then(|t| t.as_u64())
                                {
                                    println!("Settled At: {settled_at} ms");
                                }
                            }
                            if let Some(completion) = pjson.get("completion") {
                                if let Some(status) =
                                    completion.get("status").and_then(|s| s.as_str())
                                {
                                    println!("Result Status: {status}");
                                }
                                if let Some(evs) =
                                    completion.get("evidence_refs").and_then(|e| e.as_array())
                                {
                                    println!("Sealed Evidence References ({}):", evs.len());
                                    for ev in evs {
                                        println!("  - {}", ev.as_str().unwrap_or_default());
                                    }
                                }
                            }
                        }
                        if state == "completed" {
                            return Ok(());
                        } else {
                            return Err(format!("workload {wid} failed on builder"));
                        }
                    }
                }
            }
        }
    }

    Err(format!("timed out waiting for proof of workload {wid}"))
}

fn run_build(
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

    // Step 4: Build source snapshot
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

    // Step 5: Run workflow — translate --action to argv via the canonical translator.
    // No silent fallback: an unknown action is a operator error, not a default.
    let argv = canonical_argv_from_build_action(&action)
        .ok_or_else(|| format!("unknown build action: {action}"))?;

    let config = BuildWorkflowConfig {
        observation_adapter: &observation_adapter,
        lease_adapter: &lease_adapter,
        // CLI has no admitted lease input yet: explicit None blocks apply;
        // it never synthesizes authority from local arguments.
        lease_guard: None,
        argv_runner: &runner,
        source_snapshot,
        recipe,
        argv,
        artifact_path,
        // CLI install is not admitted by this surface; digest read-back only.
        install_path: None,
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

fn serve_webhook(bind: &str, node_webhook_url: &str, json: bool) -> Result<(), String> {
    use cynic_host::forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};

    if node_webhook_url.trim().is_empty() {
        return Err("node webhook URL is required; no forwarding target is guessed".to_string());
    }
    let config = WebhookServerConfig {
        bind: bind.to_string(),
        node_webhook_url: node_webhook_url.to_string(),
        webhook_secret: cynic_secrets::env_var("webhook-secret", "CYNIC_WEBHOOK_SECRET"),
    };

    let handle = spawn_webhook_server(config)
        .map_err(|e| format!("failed to bind webhook server on {bind}: {e}"))?;

    let startup = serde_json::json!({
        "status": "listening",
        "bind": bind,
        "node_webhook_url": node_webhook_url,
        "forwarding": "configured",
        "webhook_secret_present": cynic_secrets::env_var_present("webhook-secret", "CYNIC_WEBHOOK_SECRET"),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&startup).unwrap());
    } else {
        println!("Forgejo webhook server listening on {bind}");
        println!("  Node webhook: {node_webhook_url}");
        println!(
            "  Webhook secret: {}",
            if cynic_secrets::env_var_present("webhook-secret", "CYNIC_WEBHOOK_SECRET") {
                "configured"
            } else {
                "absent (unsigned pushes are reported as such)"
            }
        );
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
