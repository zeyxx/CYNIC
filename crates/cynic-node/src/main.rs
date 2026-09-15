// Binary crate test exemption — same pattern as cynic-kernel lib.rs
#![allow(dead_code)]
#![allow(
    clippy::needless_borrows_for_generic_args,
    clippy::uninlined_format_args,
    clippy::unwrap_used,
    clippy::collapsible_if,
    clippy::implicit_clone,
    clippy::needless_pass_by_value,
    clippy::single_match,
    clippy::unused_self,
    missing_debug_implementations,
    async_fn_in_trait,
    clippy::too_many_arguments,
    clippy::redundant_clone,
    clippy::manual_contains,
    clippy::needless_borrow,
    clippy::print_stdout
)]

mod announce;
mod build_attestation;
#[path = "usecases/build_scheduler/mod.rs"]
mod build_scheduler;
mod builder_activation;
mod cli;
mod config;
mod config_watch;
mod dns_activation_status;
mod dns_bundle;
mod dns_contract;
mod dns_plan;
mod network_activation_chain;
mod network_actuator;
mod network_actuator_contract;
mod network_actuator_status;
mod network_command_surface;
mod network_diagnostic_surface;
mod network_evidence;
mod network_host_substrate;
mod network_mesh_apply_plan;
mod network_mesh_bundle;
mod network_mesh_plan;
mod network_repair;
mod network_root_policy;
mod network_runtime_proof;
mod network_runtime_status;
mod network_shadow_proof;
mod network_surface_inventory;
mod node_control_runtime;
#[path = "usecases/node_enrollment_lifecycle_uc.rs"]
mod node_enrollment_lifecycle_uc;
mod observe;
mod provider_fallback_hosts;
mod provider_fallback_status;
mod runtime_supervision;
mod shutdown;
mod source_change_transaction;
mod supervise;
mod system_probe;
#[allow(dead_code, unreachable_pub)]
mod verify;
mod watchdog;
mod websocket;

// Hexagonal Architecture Modules
pub mod adapters;
pub mod domain;
pub mod ports;
pub mod usecases;

use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands};
use crate::usecases::agent_scheduler_step_uc;
use crate::usecases::agent_source_change_uc;
use crate::usecases::agent_workload_store_uc;
use crate::usecases::agent_workspace_lifecycle_uc;
use crate::usecases::build_workload_grant_uc;
use crate::usecases::network_surfaces_uc;
use crate::usecases::node_identity_uc;
use crate::usecases::source_admission_uc;
use crate::usecases::source_materialization_uc;
use crate::usecases::source_repository_bootstrap_uc;
use crate::usecases::storage_lifecycle_uc;
use shutdown::install_signal_handler;

fn apply_dns_activation_value(
    registry: &str,
    bundle_dir: &str,
    yes: bool,
) -> Result<serde_json::Value, String> {
    cynic_node::apply_dns_activation_value(registry, bundle_dir, yes)
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::NodeIdentity {
            node_id,
            signing_key_path,
            apply,
            json: _,
        }) => {
            return node_identity_uc::execute_node_identity(&node_id, &signing_key_path, apply);
        }
        Some(Commands::ConsensusStatus {
            json: _,
            expected_epoch,
            expected_commit,
            max_skew_ms,
        }) => {
            let registered_nodes = vec![
                "cynic-host-cortex".to_string(),
                "cynic-builder".to_string(),
                "cynic-host-proxmox".to_string(),
            ];
            let now_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            // Local node attestation
            let local_attestation = cynic_consensus::NodeEpochAttestation {
                schema: cynic_consensus::FLEET_EPOCH_ATTESTATION_SCHEMA.to_string(),
                node_id: "cynic-host-cortex".to_string(),
                epoch_sequence: expected_epoch,
                commit_sha: expected_commit.clone(),
                binary_sha256: "local-active".to_string(),
                registry_digest: "registry-v1".to_string(),
                monotonic_tick: 1,
                signature_hex: "local-sig".to_string(),
                attested_at_unix_ms: now_unix_ms,
            };

            let consensus = cynic_consensus::evaluate_fleet_consensus(
                expected_epoch,
                &expected_commit,
                &registered_nodes,
                &[local_attestation],
                max_skew_ms,
                now_unix_ms,
            );

            println!(
                "{}",
                serde_json::to_string_pretty(&consensus).unwrap_or_default()
            );
            return ExitCode::SUCCESS;
        }
        Some(Commands::Control { config, check }) => {
            return node_control_runtime::run(&config, check).await;
        }
        Some(Commands::Observe {
            json: _,
            registry,
            fail_on_warning,
            fail_on_critical,
        }) => {
            return network_surfaces_uc::execute_observe(
                &registry,
                fail_on_warning,
                fail_on_critical,
            );
        }
        Some(Commands::RegistryObservation {
            json: _,
            registry,
            output,
        }) => {
            return network_surfaces_uc::execute_registry_observation(&registry, output.as_deref());
        }
        Some(Commands::Network { command }) => {
            return network_surfaces_uc::execute_network_command(&command);
        }
        Some(Commands::AgentWorkspace {
            request,
            result_file,
            registry,
            state_root,
            kernel_endpoint,
            max_claim_ttl_ms,
            max_retention_ms,
            apply,
            json: _,
        }) => {
            return agent_source_change_uc::execute_agent_source_change(
                &request,
                result_file.as_deref(),
                &registry,
                &state_root,
                &kernel_endpoint,
                max_claim_ttl_ms,
                max_retention_ms,
                apply,
            )
            .await;
        }
        Some(Commands::AgentWorkspaceLifecycle {
            workload_id,
            registry,
            state_root,
            kernel_endpoint,
            agent_id,
            renewal_window_ms,
            renewal_ttl_ms,
            apply,
            json: _,
        }) => {
            return agent_workspace_lifecycle_uc::execute_agent_workspace_lifecycle(
                &workload_id,
                &registry,
                &state_root,
                &kernel_endpoint,
                &agent_id,
                renewal_window_ms,
                renewal_ttl_ms,
                apply,
            )
            .await;
        }
        Some(Commands::AgentSchedulerStep {
            observation,
            state_root,
            json: _,
        }) => {
            return agent_scheduler_step_uc::execute_agent_scheduler_step(
                &observation,
                &state_root,
            );
        }
        Some(Commands::AgentWorkloadStatus {
            workload_id,
            state_root,
            json: _,
        }) => {
            return agent_workload_store_uc::execute_agent_workload_status(
                &workload_id,
                &state_root,
            );
        }
        Some(Commands::AgentWorkloadTransition {
            request,
            state_root,
            apply,
            json: _,
        }) => {
            return agent_workload_store_uc::execute_agent_workload_transition(
                &request,
                &state_root,
                apply,
            );
        }
        Some(Commands::BuildWorkloadGrantVerify {
            grant,
            workload_record,
            authority_public_key,
            node_id,
            source_commit,
            action,
            json: _,
        }) => {
            return build_workload_grant_uc::execute_build_workload_grant_verify(
                &grant,
                &workload_record,
                &authority_public_key,
                &node_id,
                &source_commit,
                &action,
            );
        }
        Some(Commands::StorageReconcile {
            state_dir,
            warmup_seconds,
            restart_window_seconds,
            max_restarts_per_window,
            apply,
            json: _,
        }) => {
            return storage_lifecycle_uc::execute_storage_reconcile(
                &state_dir,
                warmup_seconds,
                restart_window_seconds,
                max_restarts_per_window,
                apply,
            )
            .await;
        }
        Some(Commands::SourceAdmissionPlan {
            request,
            observation,
            json: _,
        }) => {
            return source_admission_uc::execute_source_admission_plan(&request, &observation);
        }
        Some(Commands::SourceMaterialize {
            request,
            state_root,
            snapshot_root,
            snapshot_base_url,
            node_id,
            apply,
            json: _,
        }) => {
            return source_materialization_uc::execute_source_materialization(
                &request,
                &state_root,
                &snapshot_root,
                snapshot_base_url.as_deref(),
                &node_id,
                apply,
            );
        }
        Some(Commands::SourceBootstrap {
            node,
            repository,
            source,
            revision,
            registry,
            state_root,
            agent_id,
            kernel_endpoint,
            apply,
            json: _,
        }) => {
            return source_repository_bootstrap_uc::execute_source_repository_bootstrap(
                &node,
                &repository,
                &source,
                &revision,
                &registry,
                &state_root,
                &agent_id,
                &kernel_endpoint,
                apply,
            )
            .await;
        }
        Some(Commands::Peer { command }) => {
            return adapters::peer_storage::ingress::execute_peer_command(&command);
        }
        None => {}
    }

    // Automatic Versioning Integration
    let git_hash = option_env!("CYNIC_BUILD_GIT_HASH").unwrap_or("unknown_hash");
    let git_branch = option_env!("CYNIC_BUILD_GIT_BRANCH").unwrap_or("unknown_branch");
    let build_date = option_env!("CYNIC_BUILD_DATE").unwrap_or("unknown_date");

    tracing::info!(
        "cynic-node v{} ({} - {}) starting up at {}",
        env!("CARGO_PKG_VERSION"),
        git_branch,
        git_hash,
        build_date
    );

    if cli.watchdog {
        let Some(config_path) = cli.config.as_deref() else {
            tracing::error!("--config is required for --watchdog");
            return ExitCode::FAILURE;
        };
        return watchdog::execute(config_path).await;
    }

    let Some(config_path) = cli.config.as_deref() else {
        tracing::error!("--config is required unless using the observe command");
        return ExitCode::FAILURE;
    };

    let cfg = match config::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("config error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.health.timeout_secs))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("HTTP client init failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let shutdown = CancellationToken::new();
    install_signal_handler(shutdown.clone());

    let node_tasks = if let Some(control) = cfg.node_control.clone() {
        match node_control_runtime::start_tasks(control, shutdown.clone()).await {
            Ok(tasks) => tasks,
            Err(error) => {
                tracing::error!("{error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        tracing::warn!("node_control is not configured; desired-state reconciliation is inactive");
        Vec::new()
    };

    let exit = runtime_supervision::run_node(&client, config_path, cfg, &shutdown).await;
    shutdown.cancel();
    for task in node_tasks {
        let _ = task.await;
    }
    exit
}
