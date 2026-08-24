pub mod agent_scheduler_step_uc;
pub mod agent_source_change_uc;
pub mod agent_workload_delivery_uc;
pub mod agent_workload_store_uc;
pub mod agent_workspace_lifecycle_uc;
pub mod build_submission_orchestrator_uc;
pub mod build_workload_grant_uc;
pub mod build_workload_observation_uc;
pub mod build_workload_submission_uc;
pub(crate) mod bootstrap_seed_uc;
pub mod builder_http;
pub mod builder_runtime;
pub mod homeostasis_uc;
pub mod mesh_sync_uc;
pub mod network_actuation_target_uc;
pub mod network_surfaces_uc;
// The federated mesh cluster declares together or not at all: probe_uc owns
// DeclaredSynapseNode/CANONICAL_SYNAPSE_NODES, resolver calls back into
// probe_uc, and spof_observe/task_dispatch both consume probe_uc. Declaring one
// member alone produced E0433 for a module that does exist.
pub mod federated_mesh_probe_uc;
pub mod federated_mesh_resolver;
pub mod federated_task_dispatch_uc;
pub mod kernel_spof_observe_uc;
pub mod node_network_recovery_uc;
pub mod node_control_plane_uc;
pub mod node_identity_uc;
pub mod observe_node_uc;
pub mod release_webhook;
pub mod runtime_artifact_deployment_uc;
pub mod runtime_artifact_release_uc;
pub mod source_admission_uc;
pub mod source_materialization_uc;
pub mod source_repository_bootstrap_uc;
pub mod storage_controller_uc;
pub mod storage_lifecycle_uc;
