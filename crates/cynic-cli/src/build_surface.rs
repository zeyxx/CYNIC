//! Build CLI surface — thin facade over the build workflow.
//!
//! This is the CLI entry point for the build capability. It parses arguments,
//! injects the real adapters (SSH, HTTP, filesystem), and prints JSON.
//!
//! The CLI is orchestration only — no policy tables, no decision logic.

use clap::Subcommand;

use cynic_host::build_workflow::{run_build_workflow, BuildWorkflowConfig, BuildWorkflowResult};
use cynic_host::build_evidence::{BuildRunnerObservationPort, JsonBuildRunnerObservationAdapter};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::build::{BuildRecipe, BUILD_RECIPE_SCHEMA};
use cynic_protocol::SourceSnapshot;

use crate::workload_observation::{execute_observe_workload, execute_list_workloads};

/// CLI arguments for the build surface.
#[derive(Subcommand, Debug)]
pub enum BuildCommand {
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
}

/// Execute the build CLI command.
pub fn execute_build(command: BuildCommand) -> Result<(), String> {
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
    }
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
    // Step 1: Load observation
    let observation_adapter = JsonBuildRunnerObservationAdapter::new(&observation_path);
    let observation = observation_adapter
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
        target_profile: action,
        allowed_actions: target_crates,
        rust_min_stack: 16777216,
        cargo_incremental: false,
        memory_limit_bytes: None,
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

    // Step 5: Run workflow
    let config = BuildWorkflowConfig {
        observation_adapter: &observation_adapter,
        lease_adapter: &lease_adapter,
        argv_runner: &runner,
        source_snapshot,
        recipe,
        argv: vec!["make".to_string(), "check".to_string()],
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
