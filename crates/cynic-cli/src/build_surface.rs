//! Build CLI surface — thin facade over modular build workflows.

use clap::Subcommand;
use cynic_types::paths::CynicPaths;
use std::path::Path;

mod archive;
mod artifact;
mod forgejo;
mod invoice;
mod pool;
mod reclaim;
mod runner;
mod services;

pub(crate) use artifact::BuildArtifactCommand;
pub(crate) use invoice::BuildProofsCommand;
pub(crate) use pool::BuildPoolCommand;
pub(crate) use services::{BuildCredentialsCommand, BuildWebhookCommand, BuildWorkloadCommand};

use invoice::{invoice_for_commit, list_proofs};
use pool::{
    evaluate_label, evaluate_local_node, local_node_id, measure_pool_from_forgejo,
    render_eligibility,
};
use reclaim::{reclaim_build_cache_local, reclaim_build_cache_remote};
use runner::{
    list_runners, prove_build, resolve_default_builder_url, run_build, submit_build, trigger_build,
};
use services::{provision_credentials, serve_webhook};

use crate::workload_observation::{execute_list_workloads, execute_observe_workload};
use archive::{archive_terminal_workloads, ArchiveReport};
use artifact::{publish_artifact, pull_artifact, release_artifact};
use forgejo::{execute_runner_token, execute_runs};

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
        /// Action to execute: check (compiles), audit (crate audit), audit-all,
        /// build, build-release, test. Names what runs; unknown verbs are refused.
        /// Note: --package is currently ignored by every action (warned).
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
        /// Named runner (node id, name, or alias) from the fleet registry,
        /// e.g. a compat builder. Ignored when --host is set; otherwise the
        /// default `build_runner` role is used.
        #[arg(long)]
        runner: Option<String>,
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
    /// Inspect build-pool membership and toolchain eligibility.
    Pool {
        #[command(subcommand)]
        command: BuildPoolCommand,
    },
    /// List pipeline proofs, most recent first.
    Proofs {
        #[command(subcommand)]
        command: BuildProofsCommand,
    },
    /// Publish or pull runtime artifacts with fail-closed digest verification.
    Artifact {
        #[command(subcommand)]
        command: BuildArtifactCommand,
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
        #[arg(long, default_value = cynic_protocol::fleet_target::BUILD_RUNNER_SSH_TARGET)]
        host: String,
        /// Runner SSH user.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Observe one workload by id across the federation (local store + SSH).
    Workload {
        #[command(subcommand)]
        command: BuildWorkloadCommand,
    },
    /// Serve the Forgejo push webhook receiver.
    Webhook {
        #[command(subcommand)]
        command: BuildWebhookCommand,
    },
    /// Provision SSH credentials for all machine_enrollment transports in the registry.
    Credentials {
        #[command(subcommand)]
        command: BuildCredentialsCommand,
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
    /// List build runners declared in the fleet registry (read-only).
    Runners {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Path to the fleet registry JSON used for runner resolution.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
    },
    /// Archive terminal workload records (dry-run + --apply).
    Store {
        /// Node state root.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Minimum age in ms for a terminal record to be eligible.
        #[arg(long, default_value_t = 86_400_000)]
        older_than_ms: u64,
        /// Execute the archive. Without this flag the surface only reports.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Reclaim the builder's reproducible build caches, per the policy declared
    /// in `infra/registry.json` (target_profile=debug, preserve release).
    Reclaim {
        /// The build runner node id as declared in `infra/registry.json`.
        #[arg(long, default_value = "cynic-builder")]
        node: String,
        /// Path to the registry.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// The builder's state root (the registry's relative paths resolve under it).
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
        state_root: String,
        /// The builder's temporary root.
        #[arg(long, default_value = "/tmp")]
        temporary_root: String,
        /// Execute the reclaim. Without this flag the surface only reports.
        #[arg(long)]
        apply: bool,
        /// Run against the local filesystem instead of dispatching over SSH.
        /// Used by the remote invocation itself, which is already on the builder.
        #[arg(long)]
        local: bool,
        #[arg(long)]
        json: bool,
    },
    /// Render the canary deploy plan for a release commit (read-only).
    /// Couche A: the plan is data — execution stays manual.
    Deploy {
        /// Exact 40-character source commit to deploy.
        #[arg(long)]
        commit: String,
        /// Path to the deploy policy (cynic.deploy_policy.v1).
        #[arg(long, default_value = "infra/deploy-policy.json")]
        policy: String,
        /// Comma-separated machines to include (default: all known).
        #[arg(long)]
        machines: Option<String>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Issue a repo-scoped Actions runner registration token via Forgejo API.
    RunnerToken {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List recent Actions workflow runs.
    Runs {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}
/// Render the canary deploy plan (read-only, couche A).
/// Eligibility is assumed-manual: the operator verifies the release verdict
/// before executing. The plan is data from the policy, nothing is applied.
fn render_deploy_plan(
    commit: &str,
    policy_path: &str,
    machines_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if commit.len() != 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("commit must be an exact 40-character SHA".to_string());
    }
    let policy_text = std::fs::read_to_string(policy_path)
        .map_err(|e| format!("read policy {policy_path}: {e}"))?;
    let policy: cynic_release::DeployPolicyContract =
        cynic_release::DeployPolicyContract::parse(&policy_text)
            .map_err(|e| format!("parse policy: {e}"))?;
    let policy_doc: serde_json::Value =
        serde_json::from_str(&policy_text).map_err(|e| format!("parse policy document: {e}"))?;
    let gate_names: Vec<String> = policy_doc
        .pointer("/per_step_gates/health")
        .and_then(|v| v.as_array())
        .map(|gates| {
            gates
                .iter()
                .filter_map(|g| g.get("name")?.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let locations_text = std::fs::read_to_string("infra/binary-locations.json")
        .map_err(|e| format!("read binary locations: {e}"))?;
    let locations: serde_json::Value = serde_json::from_str(&locations_text)
        .map_err(|e| format!("parse binary locations: {e}"))?;
    let mut known: Vec<String> = locations
        .get("machines")
        .and_then(|v| v.as_array())
        .map(|ms| {
            ms.iter()
                .filter_map(|m| m.get("hostname")?.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if let Some(filter) = machines_filter {
        let wanted: Vec<String> = filter.split(',').map(|s| s.trim().to_string()).collect();
        known.retain(|m| wanted.contains(m));
    }
    // Couche A: eligibility verified manually via the release verdict.
    let verdict = cynic_release::ReleaseEligibilityVerdict::Eligible {
        schema: "cynic.release_verdict.v1",
        version: cynic_release::ReleaseVersion::new(0, 0, 0, None),
        commit_sha: commit.to_string(),
        tag_name: "assumed-manual-couche-a".to_string(),
        decided_at_unix_s: 0,
    };
    let plan = cynic_release::decide_deploy_plan(&verdict, &policy, &known, &gate_names, commit)?;
    if json {
        let text = serde_json::to_string_pretty(&plan)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
        println!("{text}");
    } else {
        println!("deploy plan for {commit} (eligibility assumed-manual):");
        for step in &plan.steps {
            println!(
                "  {}. {} (gates: {})",
                step.order + 1,
                step.machine,
                step.gate_names.join(", ")
            );
        }
        println!("(read-only — verify the release verdict, then execute manually)");
    }
    Ok(())
}

/// Render the archive report (JSON or text).
fn render_archive_report(report: &ArchiveReport, json: bool) {
    if json {
        let text = serde_json::to_string_pretty(report)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialize: {e}\"}}"));
        println!("{text}");
    } else {
        println!(
            "archive — eligible={} applied={} mode={}",
            report.eligible_count,
            report.archived.len(),
            if report.apply { "apply" } else { "dry-run" }
        );
        for e in &report.eligible {
            println!("  eligible: {} (age {} ms)", e.workload_id, e.age_ms);
        }
        for a in &report.archived {
            println!(
                "  archived: {} -> {} (source absent: {})",
                a.workload_id, a.archive_path, a.source_absent
            );
        }
        if !report.apply {
            println!("(dry-run — pass --apply to execute)");
        }
    }
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
            runner,
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
            runner.as_deref(),
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
        BuildCommand::Invoice {
            commit,
            proof_log,
            invoice_log,
            json,
        } => invoice_for_commit(&commit, proof_log.as_deref(), invoice_log.as_deref(), json),
        BuildCommand::Pool { command } => match command {
            BuildPoolCommand::Eligibility {
                node,
                label,
                observed_labels,
                registry,
                json,
            } => {
                // A runner knows the label it serves, not its registry id.
                let outcome = match (&label, &node) {
                    (Some(label), _) => evaluate_label(&registry, label, &observed_labels),
                    (None, Some(node)) => evaluate_local_node(&registry, node, &observed_labels),
                    (None, None) => {
                        let node_id = local_node_id();
                        evaluate_local_node(&registry, &node_id, &observed_labels)
                    }
                };
                match outcome {
                    Ok(report) => {
                        if json {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&report)
                                    .map_err(|error| error.to_string())?
                            );
                        } else {
                            render_eligibility(&report);
                        }
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
            BuildPoolCommand::Metrics {
                label,
                limit,
                registry,
                json,
            } => {
                measure_pool_from_forgejo(std::path::Path::new("."), &registry, &label, limit, json)
            }
        },
        BuildCommand::Proofs { command } => match command {
            BuildProofsCommand::List {
                commit,
                limit,
                proof_log,
                json,
            } => list_proofs(commit.as_deref(), limit, proof_log.as_deref(), json),
        },
        BuildCommand::Artifact { command } => match command {
            BuildArtifactCommand::Publish {
                artifact,
                remote_path,
                source_commit,
                publisher,
                output,
                json,
            } => publish_artifact(
                &artifact,
                &remote_path,
                &source_commit,
                &publisher,
                &output,
                json,
            ),
            BuildArtifactCommand::Pull {
                manifest,
                url,
                output,
                token_file,
                json,
            } => pull_artifact(&manifest, &url, &output, &token_file, json),
            BuildArtifactCommand::Release {
                artifact,
                manifest,
                repo,
                tag,
                api_base,
                token_file,
                json,
            } => release_artifact(
                &artifact,
                &manifest,
                &repo,
                &tag,
                &api_base,
                &token_file,
                json,
            ),
        },
        BuildCommand::Workload { command } => match command {
            BuildWorkloadCommand::Observe {
                workload_id,
                state_root,
                local,
                registry,
                user,
                json,
            } => execute_observe_workload(&workload_id, &state_root, local, &registry, &user, json),
            BuildWorkloadCommand::List {
                state_root,
                local,
                registry,
                user,
                json,
            } => execute_list_workloads(&state_root, local, &registry, &user, json),
        },
        BuildCommand::Webhook { command } => match command {
            BuildWebhookCommand::Serve {
                bind,
                node_webhook_url,
                json,
            } => serve_webhook(&bind, &node_webhook_url, json),
        },
        BuildCommand::Credentials { command } => match command {
            BuildCredentialsCommand::Provision {
                registry,
                secrets_dir,
                apply,
                json,
            } => provision_credentials(&registry, &secrets_dir, apply, json),
        },
        BuildCommand::Discover { json, kind, target } => {
            let code = crate::build_discovery_surface::execute_build_discover(
                json,
                &kind,
                target.as_deref(),
            );
            match code {
                c if c == std::process::ExitCode::SUCCESS => Ok(()),
                _ => Err("build discover failed".to_string()),
            }
        }
        BuildCommand::Runners { json, registry } => list_runners(&registry, json),
        BuildCommand::Proof {
            commit,
            package,
            phase,
            filter,
            host,
            user,
            json,
        } => prove_build(&commit, package, &phase, filter, &host, &user, json),
        BuildCommand::Store {
            state_root,
            older_than_ms,
            apply,
            json,
        } => {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let report = archive_terminal_workloads(&state_root, older_than_ms, now_ms, apply)?;
            render_archive_report(&report, json);
            Ok(())
        }
        BuildCommand::Reclaim {
            node,
            registry,
            state_root,
            temporary_root,
            apply,
            local,
            json,
        } => {
            if local {
                reclaim_build_cache_local(
                    &node,
                    &registry,
                    &state_root,
                    &temporary_root,
                    apply,
                    json,
                )
            } else {
                reclaim_build_cache_remote(
                    &node,
                    &registry,
                    &state_root,
                    &temporary_root,
                    apply,
                    json,
                )
            }
        }
        BuildCommand::RunnerToken { json } => {
            let code = execute_runner_token(Path::new("."), json);
            match code {
                c if c == std::process::ExitCode::SUCCESS => Ok(()),
                _ => Err("build runner-token failed".to_string()),
            }
        }
        BuildCommand::Deploy {
            commit,
            policy,
            machines,
            json,
        } => render_deploy_plan(&commit, &policy, machines.as_deref(), json),
        BuildCommand::Runs { json } => {
            let code = execute_runs(Path::new("."), json);
            match code {
                c if c == std::process::ExitCode::SUCCESS => Ok(()),
                _ => Err("build runs failed".to_string()),
            }
        }
    }
}
