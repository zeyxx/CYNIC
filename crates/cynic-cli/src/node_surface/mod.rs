//! Node CLI surface — unified node command dispatch.

pub mod activation;
pub mod admin;
pub mod agent;
pub mod consensus;
pub mod identity;
pub mod observation;
pub mod source;
pub mod storage;
pub mod host;
pub mod compute;
pub mod service;

use std::process::ExitCode;
use clap::Subcommand;
use cynic_types::paths::CynicPaths;

#[derive(Subcommand)]
pub enum NodeCommand {
    Status { #[arg(long)] node_id: String, #[arg(long)] json: bool },
    Activate { #[arg(long)] node_id: String, #[arg(long, default_value_t = false)] yes: bool, #[arg(long, default_value_t = false)] json: bool },
    Identity { #[command(subcommand)] command: NodeIdentityCommand },
    Csr { #[command(subcommand)] command: NodeCsrCommand },
    Cert { #[command(subcommand)] command: NodeCertCommand },
    Enrollment { #[command(subcommand)] command: NodeEnrollmentCommand },
    Forensics { #[command(subcommand)] command: crate::forensics_surface::ForensicsCommand },
    Consensus { #[command(subcommand)] command: NodeConsensusCommand },
    Observe { #[arg(long)] json: bool, #[arg(long, default_value_t = CynicPaths::registry_path().display().to_string())] registry: String, #[arg(long)] fail_on_warning: bool, #[arg(long)] fail_on_critical: bool },
    Registry { #[command(subcommand)] command: NodeRegistryCommand },
    Host { #[command(subcommand)] command: host::NodeHostCommand },
    Compute { #[command(subcommand)] command: compute::NodeComputeCommand },
    Peer { #[command(subcommand)] command: cynic_node::peer_commands::PeerCommands },
    Agent { #[command(subcommand)] command: NodeAgentCommand },
    Build { #[command(subcommand)] command: NodeBuildCommand },
    Storage { #[command(subcommand)] command: NodeStorageCommand },
    Source { #[command(subcommand)] command: NodeSourceCommand },
    Service { #[command(subcommand)] command: NodeServiceCommand },
    Admin { #[command(subcommand)] command: admin::NodeAdminCommand },
}

#[derive(Subcommand)]
pub enum NodeIdentityCommand {
    Init { #[arg(long)] key_path: Option<String>, #[arg(long)] json: bool },
    Run { #[arg(long)] node_id: String, #[arg(long, default_value = "/var/lib/cynic/identity/node-signing.key")] signing_key_path: String, #[arg(long)] apply: bool, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeCsrCommand { Create { #[arg(long)] node_id: String, #[arg(long)] role: String, #[arg(long)] key_path: Option<String>, #[arg(long)] json: bool } }

#[derive(Subcommand)]
pub enum NodeCertCommand {
    Inspect { #[arg(long)] cert_path: Option<String>, #[arg(long)] json: bool },
    Verify { #[arg(long)] kernel_pubkey_hex: String, #[arg(long)] cert_path: Option<String>, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeEnrollmentCommand {
    Check { #[arg(long, default_value = "/var/lib/cynic/node-control.toml")] config: String, #[arg(long)] json: bool },
    Issue { #[arg(long)] request: String, #[arg(long)] authority: String, #[arg(long)] token_file: Option<String>, #[arg(long)] signing_key: String, #[arg(long)] output: Option<String>, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeConsensusCommand {
    Status { #[arg(long)] json: bool, #[arg(long, default_value_t = 1)] expected_epoch: u64, #[arg(long, default_value = "HEAD")] expected_commit: String, #[arg(long, default_value_t = 5000)] max_skew_ms: u64 },
}

#[derive(Subcommand)]
pub enum NodeRegistryCommand {
    Observation { #[arg(long)] json: bool, #[arg(long, default_value = "/etc/cynic/registry.json")] registry: String, #[arg(long)] output: Option<String> },
}

#[derive(Subcommand)]
pub enum NodeAgentCommand {
    Scheduler { #[command(subcommand)] command: NodeAgentSchedulerCommand },
    Workload { #[command(subcommand)] command: NodeAgentWorkloadCommand },
    Workspace {
        #[arg(long)] request: Option<String>,
        #[arg(long)] result_file: Option<String>,
        #[arg(long, default_value = "/etc/cynic/registry.json")] registry: String,
        #[arg(long, default_value = "/var/lib/cynic")] state_root: String,
        #[arg(long, default_value = "http://127.0.0.1:3030")] kernel_endpoint: String,
        #[arg(long, default_value_t = 3_600_000)] max_claim_ttl_ms: u64,
        #[arg(long, default_value_t = 604_800_000)] max_retention_ms: u64,
        #[arg(long)] apply: bool,
        #[arg(long)] json: bool,
        #[command(subcommand)] command: Option<NodeAgentWorkspaceLifecycleCommand>,
    },
}

#[derive(Subcommand)]
pub enum NodeAgentSchedulerCommand {
    Observe { #[arg(long)] node_id: String, #[arg(long)] scheduler_epoch: u64, #[arg(long)] step_id: String, #[arg(long)] budgets: String, #[arg(long)] lifecycle: String, #[arg(long)] output: String, #[arg(long)] json: bool },
    Step { #[arg(long)] observation: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeAgentWorkloadCommand {
    Status { #[arg(long)] workload_id: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long)] json: bool },
    Transition { #[arg(long)] request: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long)] apply: bool, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeAgentWorkspaceLifecycleCommand {
    Lifecycle { #[arg(long)] workload_id: String, #[arg(long, default_value = "/etc/cynic/registry.json")] registry: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long, default_value = "http://127.0.0.1:3030")] kernel_endpoint: String, #[arg(long)] agent_id: String, #[arg(long, default_value_t = 300_000)] renewal_window_ms: u64, #[arg(long, default_value_t = 3_600_000)] renewal_ttl_ms: u64, #[arg(long)] apply: bool, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeBuildCommand { Workload { #[command(subcommand)] command: NodeBuildWorkloadCommand } }

#[derive(Subcommand)]
pub enum NodeBuildWorkloadCommand { Grant { #[command(subcommand)] command: NodeBuildWorkloadGrantCommand } }

#[derive(Subcommand)]
pub enum NodeBuildWorkloadGrantCommand { Verify { #[arg(long)] grant: String, #[arg(long)] workload_record: String, #[arg(long)] authority_public_key: String, #[arg(long)] node_id: String, #[arg(long)] source_commit: String, #[arg(long)] action: String, #[arg(long)] json: bool } }

#[derive(Subcommand)]
pub enum NodeStorageCommand { Reconcile { #[arg(long, default_value = "/var/lib/cynic/storage-lifecycle")] state_dir: String, #[arg(long, default_value_t = 30)] warmup_seconds: u64, #[arg(long, default_value_t = 300)] restart_window_seconds: u64, #[arg(long, default_value_t = 1)] max_restarts_per_window: u32, #[arg(long)] apply: bool, #[arg(long)] json: bool } }

#[derive(Subcommand)]
pub enum NodeSourceCommand {
    Admission { #[command(subcommand)] command: NodeSourceAdmissionCommand },
    Bootstrap { #[arg(long)] node: String, #[arg(long)] repository: String, #[arg(long)] source: String, #[arg(long)] revision: String, #[arg(long, default_value = "/etc/cynic/registry.json")] registry: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long)] agent_id: String, #[arg(long, default_value = "http://127.0.0.1:3030")] kernel_endpoint: String, #[arg(long)] apply: bool, #[arg(long)] json: bool },
    Materialize { #[arg(long)] request: String, #[arg(long, default_value = "/var/lib/cynic")] state_root: String, #[arg(long, default_value = "/var/lib/cynic/source/snapshots")] snapshot_root: String, #[arg(long)] snapshot_base_url: Option<String>, #[arg(long)] node_id: String, #[arg(long)] apply: bool, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
pub enum NodeSourceAdmissionCommand { Plan { #[arg(long)] request: String, #[arg(long)] observation: String, #[arg(long)] json: bool } }

#[derive(Subcommand)]
pub enum NodeServiceCommand {
    Generate { #[arg(long, default_value = "cynic-node.service")] unit_name: String, #[arg(long, default_value = "CYNIC node receiver")] description: String, #[arg(long)] binary: String, #[arg(long)] config: String, #[arg(long, default_value = "cynic")] user: String, #[arg(long)] output: Option<String>, #[arg(long)] json: bool },
    Prove { #[arg(long, default_value = "cynic-node.service")] unit_name: String, #[arg(long, default_value = "CYNIC node receiver")] description: String, #[arg(long)] binary: String, #[arg(long)] config: String, #[arg(long, default_value = "cynic")] user: String, #[arg(long, default_value = "/etc/systemd/system")] unit_dir: String, #[arg(long)] json: bool },
}

pub async fn execute_node(command: NodeCommand) -> ExitCode {
    match command {
        NodeCommand::Status { node_id, json } => activation::execute_status(node_id, json),
        NodeCommand::Activate { node_id, yes, json } => activation::execute_activate(node_id, yes, json),
        NodeCommand::Identity { command } => match command {
            NodeIdentityCommand::Init { key_path, json } => activation::execute_identity_init(key_path, json),
            NodeIdentityCommand::Run { node_id, signing_key_path, apply, json } => identity::execute_node_identity(&node_id, &signing_key_path, apply, json),
        },
        NodeCommand::Csr { command } => match command { NodeCsrCommand::Create { node_id, role, key_path, json } => activation::execute_csr_create(node_id, role, key_path, json) },
        NodeCommand::Cert { command } => match command {
            NodeCertCommand::Inspect { cert_path, json } => activation::execute_cert_inspect(cert_path, json),
            NodeCertCommand::Verify { kernel_pubkey_hex, cert_path, json } => activation::execute_cert_verify(kernel_pubkey_hex, cert_path, json),
        },
        NodeCommand::Enrollment { command } => match command {
            NodeEnrollmentCommand::Check { config, json } => identity::execute_node_enrollment_check(&config, json),
            NodeEnrollmentCommand::Issue { request, authority, token_file, signing_key, output, json } => activation::execute_enrollment_issue(request, authority, token_file, signing_key, output, json),
        },
        NodeCommand::Forensics { command } => crate::forensics_surface::execute_forensics(command),
        NodeCommand::Consensus { command } => match command { NodeConsensusCommand::Status { json: _, expected_epoch, expected_commit, max_skew_ms } => consensus::execute_consensus_status(expected_epoch, &expected_commit, max_skew_ms) },
        NodeCommand::Observe { json: _, registry, fail_on_warning, fail_on_critical } => observation::execute_observe(&registry, fail_on_warning, fail_on_critical),
        NodeCommand::Registry { command } => match command { NodeRegistryCommand::Observation { json: _, registry, output } => observation::execute_registry_observation(&registry, output.as_deref()) },
        NodeCommand::Host { command } => host::execute_node_host(command),
        NodeCommand::Compute { command } => compute::dispatch(command).await,
        NodeCommand::Peer { command } => cynic_node::adapters::peer_storage::ingress::execute_peer_command(&command),
        NodeCommand::Agent { command } => match command {
            NodeAgentCommand::Scheduler { command } => match command {
                NodeAgentSchedulerCommand::Observe { node_id, scheduler_epoch, step_id, budgets, lifecycle, output, json: _ } => agent::execute_agent_scheduler_observe(&node_id, scheduler_epoch, &step_id, &budgets, &lifecycle, &output),
                NodeAgentSchedulerCommand::Step { observation, state_root, json: _ } => agent::execute_agent_scheduler_step(&observation, &state_root),
            },
            NodeAgentCommand::Workload { command } => match command {
                NodeAgentWorkloadCommand::Status { workload_id, state_root, json: _ } => agent::execute_agent_workload_status(&workload_id, &state_root),
                NodeAgentWorkloadCommand::Transition { request, state_root, apply, json: _ } => agent::execute_agent_workload_transition(&request, &state_root, apply),
            },
            NodeAgentCommand::Workspace { request, result_file, registry, state_root, kernel_endpoint, max_claim_ttl_ms, max_retention_ms, apply, json: _, command } => {
                if let Some(lc) = command {
                    match lc {
                        NodeAgentWorkspaceLifecycleCommand::Lifecycle { workload_id, registry, state_root, kernel_endpoint, agent_id, renewal_window_ms, renewal_ttl_ms, apply, json: _ } => agent::execute_agent_workspace_lifecycle(&workload_id, &registry, &state_root, &kernel_endpoint, &agent_id, renewal_window_ms, renewal_ttl_ms, apply).await,
                    }
                } else {
                    match request {
                        Some(req) => agent::execute_agent_workspace(&req, result_file.as_deref(), &registry, &state_root, &kernel_endpoint, max_claim_ttl_ms, max_retention_ms, apply).await,
                        None => {
                            eprintln!("error: --request required");
                            ExitCode::FAILURE
                        }
                    }
                }
            }
        },
        NodeCommand::Build { command } => match command { NodeBuildCommand::Workload { command } => match command { NodeBuildWorkloadCommand::Grant { command } => match command { NodeBuildWorkloadGrantCommand::Verify { grant, workload_record, authority_public_key, node_id, source_commit, action, json: _ } => agent::execute_build_workload_grant_verify(&grant, &workload_record, &authority_public_key, &node_id, &source_commit, &action) } } },
        NodeCommand::Storage { command } => match command { NodeStorageCommand::Reconcile { state_dir, warmup_seconds, restart_window_seconds, max_restarts_per_window, apply, json: _ } => storage::execute_storage_reconcile(&state_dir, warmup_seconds, restart_window_seconds, max_restarts_per_window, apply).await },
        NodeCommand::Source { command } => match command {
            NodeSourceCommand::Admission { command } => match command { NodeSourceAdmissionCommand::Plan { request, observation, json: _ } => source::execute_source_admission_plan(&request, &observation) },
            NodeSourceCommand::Bootstrap { node, repository, source, revision, registry, state_root, agent_id, kernel_endpoint, apply, json: _ } => source::execute_source_bootstrap(&node, &repository, &source, &revision, &registry, &state_root, &agent_id, &kernel_endpoint, apply).await,
            NodeSourceCommand::Materialize { request, state_root, snapshot_root, snapshot_base_url, node_id, apply, json: _ } => source::execute_source_materialize(&request, &state_root, &snapshot_root, snapshot_base_url.as_deref(), &node_id, apply),
        },
        NodeCommand::Service { command } => match command {
            NodeServiceCommand::Generate { unit_name, description, binary, config, user, output, json } => service::execute_service_generate(unit_name, description, binary, config, user, output, json),
            NodeServiceCommand::Prove { unit_name, description, binary, config, user, unit_dir, json } => service::execute_service_prove(unit_name, description, binary, config, user, unit_dir, json),
        },
        NodeCommand::Admin { command } => match command {
            admin::NodeAdminCommand::Secret { command } => match admin::execute_admin_secret(&command) {
                Ok(()) => std::process::ExitCode::SUCCESS,
                Err(error) => { eprintln!("error: {error}"); std::process::ExitCode::FAILURE }
            },
        },
    }
}
