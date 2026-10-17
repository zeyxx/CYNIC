//! Build CLI surface — thin facade over modular build workflows.

use clap::Subcommand;
use cynic_types::paths::CynicPaths;

mod archive;
mod invoice;
mod reclaim;
mod runner;
mod services;

pub(crate) use invoice::BuildProofsCommand;
pub(crate) use services::{BuildCredentialsCommand, BuildWebhookCommand, BuildWorkloadCommand};

use invoice::{invoice_for_commit, list_proofs};
use reclaim::{reclaim_build_cache_local, reclaim_build_cache_remote};
use runner::{prove_build, resolve_default_builder_url, run_build, submit_build, trigger_build};
use services::{provision_credentials, serve_webhook};

use crate::workload_observation::{execute_list_workloads, execute_observe_workload};
use archive::{ArchiveReport, archive_terminal_workloads};

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
    /// List pipeline proofs, most recent first.
    Proofs {
        #[command(subcommand)]
        command: BuildProofsCommand,
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
        BuildCommand::Invoice {
            commit,
            proof_log,
            invoice_log,
            json,
        } => invoice_for_commit(&commit, proof_log.as_deref(), invoice_log.as_deref(), json),
        BuildCommand::Proofs { command } => match command {
            BuildProofsCommand::List {
                commit,
                limit,
                proof_log,
                json,
            } => list_proofs(commit.as_deref(), limit, proof_log.as_deref(), json),
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
    }
}
