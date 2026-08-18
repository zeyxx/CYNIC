//! Thin CLI projection for the admitted bootstrap build workload.
//!
//! The command owns argument decoding and port injection only. Admission,
//! source read-back, build execution, outcome persistence, and proof shape stay
//! in the host/framework boundaries.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::{
    BuildRunnerBootstrapCapabilityBinding, OneShotCapabilityOutcome,
    OneShotCapabilityOutcomeStatus, build_runner_bootstrap_action_digest,
    validate_build_runner_bootstrap_capability_binding,
};
use sha2::{Digest, Sha256};

use crate::framework_bootstrap::build_runner::{
    BootstrapBuildProof, BuildRunnerBootstrapRequest, apply_build_runner_bootstrap,
    apply_build_runner_bootstrap_build, bootstrap_admission_from_proof, bootstrap_build_proof,
    build_runner_bootstrap_build_plan, build_runner_bootstrap_execution_plan,
    observe_build_runner_bootstrap, reconcile_build_runner_bootstrap,
    write_bootstrap_admission_receipt, write_bootstrap_build_proof,
};
use crate::one_shot_authority::{LocalOneShotCapabilityAuthority, OneShotCapabilityAuthorityPort};
use crate::toolchain::ToolchainLifecycle;
use crate::update_lifecycle::{ArgvRunner, LocalArgvRunner};

use super::{args, io};

pub(crate) fn execute(args: &[String]) -> Result<std::process::ExitCode, String> {
    let binding_path = required(args, "--binding")?;
    let authority_public_key = required(args, "--authority-public-key")?;
    let state_root = required(args, "--state-root")?;
    let source_bundle = required(args, "--source-bundle")?;
    let registry = required(args, "--registry")?;
    let apply = args::has_flag(args, "--apply");
    let now = now_unix_s()?;
    let binding: BuildRunnerBootstrapCapabilityBinding = read_json(&binding_path, "binding")?;
    let action = "make:build-host-exec";
    let binding_blockers =
        validate_build_runner_bootstrap_capability_binding(&binding, action, now);
    if !binding_blockers.is_empty() {
        return Err(binding_blockers.join(","));
    }

    let authority = LocalOneShotCapabilityAuthority::new(&authority_public_key, &state_root);
    let recovered = authority.recover_unfinished_consumptions(now)?;
    if !recovered.is_empty() {
        return Err(format!(
            "build_runner_bootstrap_unknown_after_crash:{}",
            recovered
                .iter()
                .map(|outcome| outcome.capability_id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !apply {
        return Err("build_runner_bootstrap_explicit_apply_required".to_string());
    }

    let workspace = controlled_workspace(&state_root, &binding.capability.capability_id)?;
    let runner = LocalArgvRunner;
    initialize_workspace(&workspace, &runner)?;
    // Toolchain lifecycle: observe -> reconcile -> bounded argv action (rustup override)
    // BEFORE cargo check, per CYNIC §3 (dynamic detection before specialization).
    let lifecycle = ToolchainLifecycle::new(crate::toolchain::observe::LocalToolchainObserver::new(
        crate::toolchain::specs::ToolchainRegistry::default(),
    ));
    let toolchain_crate_name = binding.node_subject.clone();
    let seed_path = Path::new("infra/rust-toolchain.json");
    let history_path = crate::toolchain::specs::ToolchainProofHistory::path(&toolchain_crate_name);
    let _tc_report = match lifecycle.run(
        &toolchain_crate_name,
        &seed_path,
        &history_path,
        &runner,
    ) {
        Ok(report) => Some(report),
        Err(_) => None,
    };
    let request = BuildRunnerBootstrapRequest {
        node_id: binding.node_subject.clone(),
        source_commit: binding.source_commit.clone(),
        source_bundle_digest: binding.source_bundle_digest.clone(),
        target_phase: "build".to_string(),
    };
    let observation = observe_build_runner_bootstrap(
        &runner,
        &workspace,
        Path::new(&registry),
        Path::new(&source_bundle),
        &binding.source_bundle_digest,
        &binding.node_subject,
        Path::new(&state_root),
        &Path::new(&state_root).join("tmp"),
    );
    let plan = reconcile_build_runner_bootstrap(&request, &observation);
    if plan.status != "planned" {
        return Err(plan.blockers.join(","));
    }

    let registry_digest = file_digest(Path::new(&registry))
        .ok_or_else(|| "build_runner_bootstrap_registry_read_failed".to_string())?;
    if registry_digest != binding.registry_digest {
        return Err("build_runner_bootstrap_registry_digest_mismatch".to_string());
    }
    let receipt_path = Path::new(&state_root).join(format!(
        "{}.admission.json",
        binding.capability.capability_id
    ));
    let admission = bootstrap_admission_from_proof(&plan, Path::new(&registry), registry_digest);
    write_bootstrap_admission_receipt(&receipt_path, &admission)?;
    let action_digest = build_runner_bootstrap_action_digest(action)
        .map_err(|_| "build_runner_bootstrap_action_digest_failed".to_string())?;
    let consumption = authority.verify_and_consume(
        &binding.capability,
        &binding.node_subject,
        "build-runner-bootstrap",
        &action_digest,
        now,
    )?;

    let result = execute_consumed_build(
        &authority,
        &consumption.capability_digest,
        &plan,
        &workspace,
        Path::new(&source_bundle),
        &receipt_path,
        &binding.registry_digest,
        &binding.capability.capability_id,
        &runner,
        now,
    );
    match result {
        Ok(proof) => io::emit_json(&proof, false),
        Err(error) => {
            let outcome = failure_outcome(
                &binding.capability.capability_id,
                &consumption.capability_digest,
                now,
                error.clone(),
            );
            authority.record_outcome(&outcome)?;
            Err(error)
        }
    }
}

fn execute_consumed_build(
    authority: &dyn OneShotCapabilityAuthorityPort,
    consumption_digest: &str,
    plan: &crate::framework_bootstrap::build_runner::BuildRunnerBootstrapPlan,
    workspace: &Path,
    source_bundle: &Path,
    receipt_path: &Path,
    expected_registry_digest: &str,
    capability_id: &str,
    runner: &dyn ArgvRunner,
    now: u64,
) -> Result<BootstrapBuildProof, String> {
    let source_plan = build_runner_bootstrap_execution_plan(plan, workspace, source_bundle)
        .ok_or_else(|| "build_runner_bootstrap_execution_plan_missing".to_string())?;
    let source_proof = apply_build_runner_bootstrap(
        &source_plan,
        &plan.source_commit,
        expected_registry_digest,
        true,
        runner,
    )?;
    let build_plan = build_runner_bootstrap_build_plan(workspace, receipt_path);
    let artifact_digest = apply_build_runner_bootstrap_build(&build_plan, runner)?;
    let mut proof = bootstrap_build_proof(plan, source_proof, &build_plan, artifact_digest);
    let proof_path = receipt_path.with_file_name(format!("{capability_id}.build-proof.json"));
    proof
        .evidence_refs
        .push(format!("bootstrap_build_proof:{}", proof_path.display()));
    write_bootstrap_build_proof(&proof_path, &proof)?;
    let outcome = OneShotCapabilityOutcome {
        schema: cynic_protocol::ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA.to_string(),
        capability_id: capability_id.to_string(),
        consumption_digest: consumption_digest.to_string(),
        status: OneShotCapabilityOutcomeStatus::Succeeded,
        recorded_at_unix_s: now,
        evidence_refs: proof.evidence_refs.clone(),
        blockers: Vec::new(),
    };
    authority.record_outcome(&outcome)?;
    Ok(proof)
}

fn failure_outcome(
    capability_id: &str,
    consumption_digest: &str,
    now: u64,
    blocker: String,
) -> OneShotCapabilityOutcome {
    OneShotCapabilityOutcome {
        schema: cynic_protocol::ONE_SHOT_CAPABILITY_OUTCOME_SCHEMA.to_string(),
        capability_id: capability_id.to_string(),
        consumption_digest: consumption_digest.to_string(),
        status: OneShotCapabilityOutcomeStatus::Failed,
        recorded_at_unix_s: now,
        evidence_refs: Vec::new(),
        blockers: vec![blocker],
    }
}

fn initialize_workspace(workspace: &Path, runner: &dyn ArgvRunner) -> Result<(), String> {
    if workspace.exists() {
        return Err("build_runner_bootstrap_workspace_already_exists".to_string());
    }
    fs::create_dir_all(workspace)
        .map_err(|error| format!("build_runner_bootstrap_workspace_create_failed:{error}"))?;
    let path = workspace.display().to_string();
    if !runner.run(&["git", "-C", &path, "init"]).success() {
        let _ = fs::remove_dir_all(workspace);
        return Err("build_runner_bootstrap_workspace_init_failed".to_string());
    }
    Ok(())
}

fn controlled_workspace(state_root: &str, capability_id: &str) -> Result<PathBuf, String> {
    let root = Path::new(state_root);
    if !root.is_absolute()
        || root
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("build_runner_bootstrap_state_root_unsafe".to_string());
    }
    Ok(root.join("workspaces").join(capability_id))
}

fn required(args: &[String], flag: &str) -> Result<String, String> {
    args::flag_value(args, flag).ok_or_else(|| format!("build-runner-bootstrap requires {flag}"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str, label: &str) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|_| format!("build_runner_bootstrap_{label}_read_failed"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| format!("build_runner_bootstrap_{label}_decode_failed"))
}

fn file_digest(path: &Path) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|bytes| format!("sha256:{}", hex::encode(Sha256::digest(bytes))))
}

fn now_unix_s() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| "build_runner_bootstrap_clock_unavailable".to_string())
}
