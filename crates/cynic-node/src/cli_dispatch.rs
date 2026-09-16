//! Subcommand execution dispatch for cynic-node.

use std::process::ExitCode;

use crate::adapters;
use crate::cli::Commands;
use crate::node_control_runtime;
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

pub async fn dispatch_command(command: Commands) -> ExitCode {
    match command {
        Commands::NodeIdentity {
            node_id,
            signing_key_path,
            apply,
            json: _,
        } => node_identity_uc::execute_node_identity(&node_id, &signing_key_path, apply),
        Commands::ConsensusStatus {
            json: _,
            expected_epoch,
            expected_commit,
            max_skew_ms,
        } => {
            let registered_nodes = vec![
                "cynic-host-cortex".to_string(),
                "cynic-builder".to_string(),
                "cynic-host-proxmox".to_string(),
            ];
            let now_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

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
            ExitCode::SUCCESS
        }
        Commands::Control { config, check } => node_control_runtime::run(&config, check).await,
        Commands::Observe {
            json: _,
            registry,
            fail_on_warning,
            fail_on_critical,
        } => {
            network_surfaces_uc::execute_observe(&registry, fail_on_warning, fail_on_critical)
        }
        Commands::RegistryObservation {
            json: _,
            registry,
            output,
        } => network_surfaces_uc::execute_registry_observation(&registry, output.as_deref()),
        Commands::Network { command } => {
            network_surfaces_uc::execute_network_command(&command)
        }
        Commands::AgentWorkspace {
            request,
            result_file,
            registry,
            state_root,
            kernel_endpoint,
            max_claim_ttl_ms,
            max_retention_ms,
            apply,
            json: _,
        } => {
            agent_source_change_uc::execute_agent_source_change(
                &request,
                result_file.as_deref(),
                &registry,
                &state_root,
                &kernel_endpoint,
                max_claim_ttl_ms,
                max_retention_ms,
                apply,
            )
            .await
        }
        Commands::AgentWorkspaceLifecycle {
            workload_id,
            registry,
            state_root,
            kernel_endpoint,
            agent_id,
            renewal_window_ms,
            renewal_ttl_ms,
            apply,
            json: _,
        } => {
            agent_workspace_lifecycle_uc::execute_agent_workspace_lifecycle(
                &workload_id,
                &registry,
                &state_root,
                &kernel_endpoint,
                &agent_id,
                renewal_window_ms,
                renewal_ttl_ms,
                apply,
            )
            .await
        }
        Commands::AgentSchedulerStep {
            observation,
            state_root,
            json: _,
        } => agent_scheduler_step_uc::execute_agent_scheduler_step(&observation, &state_root),
        Commands::AgentWorkloadStatus {
            workload_id,
            state_root,
            json: _,
        } => agent_workload_store_uc::execute_agent_workload_status(&workload_id, &state_root),
        Commands::AgentWorkloadTransition {
            request,
            state_root,
            apply,
            json: _,
        } => agent_workload_store_uc::execute_agent_workload_transition(
            &request,
            &state_root,
            apply,
        ),
        Commands::BuildWorkloadGrantVerify {
            grant,
            workload_record,
            authority_public_key,
            node_id,
            source_commit,
            action,
            json: _,
        } => build_workload_grant_uc::execute_build_workload_grant_verify(
            &grant,
            &workload_record,
            &authority_public_key,
            &node_id,
            &source_commit,
            &action,
        ),
        Commands::StorageReconcile {
            state_dir,
            warmup_seconds,
            restart_window_seconds,
            max_restarts_per_window,
            apply,
            json: _,
        } => {
            storage_lifecycle_uc::execute_storage_reconcile(
                &state_dir,
                warmup_seconds,
                restart_window_seconds,
                max_restarts_per_window,
                apply,
            )
            .await
        }
        Commands::SourceAdmissionPlan {
            request,
            observation,
            json: _,
        } => source_admission_uc::execute_source_admission_plan(&request, &observation),
        Commands::SourceMaterialize {
            request,
            state_root,
            snapshot_root,
            snapshot_base_url,
            node_id,
            apply,
            json: _,
        } => source_materialization_uc::execute_source_materialization(
            &request,
            &state_root,
            &snapshot_root,
            snapshot_base_url.as_deref(),
            &node_id,
            apply,
        ),
        Commands::SourceBootstrap {
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
        } => {
            source_repository_bootstrap_uc::execute_source_repository_bootstrap(
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
            .await
        }
        Commands::Peer { command } => {
            adapters::peer_storage::ingress::execute_peer_command(&command)
        }
    }
}
