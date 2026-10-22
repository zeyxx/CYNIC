pub mod agent_host_placement_uc;
pub mod compute_provisioning_uc;
pub mod agent_scheduler_step_uc;
pub mod agent_source_change_uc;
pub mod agent_workload_delivery_uc;
pub mod agent_workload_store_uc;
pub mod agent_workspace_lifecycle_uc;
pub mod bootstrap_seed_uc;
pub mod build_admission;
/// WHY: Executes isolated pull and remote compilation on runner nodes.
/// HOW: Invoked by build_scheduler to dequeue and run admitted workloads.
pub mod build_pull;
pub mod build_submission_orchestrator_uc;
pub mod build_workload_grant_uc;
pub mod build_workload_observation_uc;
pub mod build_workload_submission_uc;
pub mod builder_http;
pub mod builder_runtime;
pub mod coordination_observation_bridge;
pub mod coordination_posture_uc;
pub mod mesh_sync_uc;
pub mod network_actuation_target_uc;
pub mod network_sync_uc;
pub mod network_watchdog_uc;
// The federated mesh cluster declares together or not at all: resolver reads the
// declared seed, and spof_observe/task_dispatch both consume the resolver.
// Declaring one member alone produced E0433 for a module that does exist.
pub mod build_scheduler;
pub mod federated_mesh_probe_uc;
pub mod federated_mesh_resolver;
pub mod federated_task_dispatch_uc;
pub mod federation_membership_uc;
/// WHY: a muted or degraded runner must reach the operator.
/// HOW: pure reconcile of fleet usability into idempotent outbox entries;
/// delivery rides the existing notify drain.
pub mod fleet_outbox_uc;
pub mod kernel_spof_observe_uc;
pub mod node_control_plane_uc;
pub mod node_enrollment_check_uc;
pub mod node_enrollment_lifecycle_uc;
pub mod node_identity_uc;
pub mod node_network_recovery_uc;
pub mod notification_outbox_uc;
pub mod observe_node_uc;
pub mod release_webhook;
pub mod remote_runner_enrollment_uc;
pub mod runtime_artifact_deployment_uc;
pub mod source_admission_uc;
pub mod source_materialization_uc;
pub mod source_repository_bootstrap_uc;
pub mod storage_controller_uc;
pub mod storage_lifecycle_uc;
pub mod agent_bus_types;
pub mod agent_bus_ledger;
pub mod agent_bus_claim;
pub mod agent_bus_ports;
pub mod agent_coordination_uc;
pub mod agent_coordination_taxonomy;
/// WHY: OQ-4 policy enforcement for collaborator trust tier changes.
/// HOW: Pure logic validating promotion/demotion requests against
/// infra/collaborators.json and infra/external-projects.json, using
/// cynic-vcs trust-tier admission rules.
pub mod collaborator_trust_uc;
/// WHY: the declared collaborator registries drift silently from the forges.
/// HOW: pure diff between observed handles and declared ids/fiches; proposals
/// only, never admission.
pub mod collaborator_watch_uc;
/// WHY: End-to-end audit orchestration for external projects.
/// HOW: Composes ExternalSourcePort → ExternalBuildPort → ExternalEvidencePort
/// in sequence: clone repo → run build → ingest evidence batch.
pub mod external_project_audit_uc;
