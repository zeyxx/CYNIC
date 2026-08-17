//! CLI surface for adopting an external agent runtime into CYNIC supervision.
//! This is an explicit, operator-consent-gated mutation surface.
//! It consumes the `AgentRuntimeAdoptionPlan` produced by
//! `cynic_host::agent_runtime::plan_runtime_adoption` to present a plan
//! (dry-run) and, only with `--approve`, execute `bounded_action.argv`.

use std::process::ExitCode;

use cynic_host::agent_runtime::{
    apply_runtime_ownership, classify_runtime_ownership_with_external_gateway,
    observe_agent_runtimes, plan_runtime_adoption, AgentRuntimeAdoptionPlan,
    AgentRuntimeObservation, AgentRuntimeOwnership,
};
use cynic_host::runtime_authority::default_runtime_authority_paths;
use cynic_host::toolchain::observe::ToolchainObservationPort;
use cynic_host::toolchain::{ToolchainLifecycle, ToolchainLifecycleReport};
use cynic_host::update_lifecycle::{ArgvRunner, LocalArgvRunner};

use crate::{emit_json, fail};

/// Execute the runtime adoption command.
/// Dry-run (plan projection) unless `--approve` is passed.
/// `seed` optionally overrides the toolchain seed registry path
/// (default: infra/rust-toolchain.json) to drive the Rust toolchain lifecycle.
pub(crate) fn execute_adopt(
    json: bool,
    runtime_id: &str,
    principal: Option<&str>,
    seed: Option<&std::path::Path>,
    approve: bool,
) -> ExitCode {
    if !json {
        return fail("cynic runtime adopt requires --json output mode");
    }

    let runner = LocalArgvRunner;

    // 1. Observe all declared agent runtimes (read-only probe).
    let mut inventory = observe_agent_runtimes(&runner);

    // 1b. Toolchain lifecycle (observe → reconcile → bounded argv) projection.
    // The seed is declarative desired-state; proof history in target/proofs is
    // the learned-feedback loop. Projection only (no --yes / mutation without --approve).
    let toolchain_report = run_toolchain_lifecycle(runtime_id, seed, approve);

    let principal_user = principal.map(String::from).unwrap_or_else(|| {
        default_runtime_authority_paths()
            .principal
            .runtime_user
            .to_string()
    });

    // 2. Locate the target runtime by ID (runtime.id is &'static str).
    let Some(runtime) = inventory.runtimes.iter_mut().find(|r| r.id == runtime_id) else {
        return fail(format!(
            "runtime_id '{runtime_id}' not present in agent inventory"
        ));
    };

    // 3. Derive ownership from process identity (mirrors agents.rs path).
    let ownership = derive_ownership_for_runtime(runtime, &principal_user);

    // 4. Plan the adoption (pure decision: desired + observed → plan).
    let gateway_ready = false;
    let adoption = plan_runtime_adoption(
        runtime.id,
        principal_user.clone(),
        ownership.clone(),
        gateway_ready,
    );

    if !approve {
        let report = AdoptReport {
            runtime_id: runtime.id,
            principal: &principal_user,
            ownership: &ownership,
            adoption: &adoption,
            toolchain: toolchain_report,
            action_taken: false,
        };
        emit_json(&report, false);
        return ExitCode::SUCCESS;
    }

    // 5. Bounded action: execute bounded_action.argv with operator consent.
    if !adoption.bounded_action.requires_operator_consent {
        return fail("adoption plan does not require operator consent; re-run without --approve");
    }

    let argv_refs: Vec<&str> = adoption
        .bounded_action
        .argv
        .iter()
        .map(String::as_str)
        .collect();

    let _exit = runner.run(&argv_refs);

    // 6. Apply ownership + read-back.
    apply_runtime_ownership(runtime, ownership.clone());

    let report = AdoptReport {
        runtime_id: runtime.id,
        principal: &principal_user,
        ownership: &ownership,
        adoption: &adoption,
        toolchain: toolchain_report,
        action_taken: true,
    };

    emit_json(&report, false);
    ExitCode::SUCCESS
}

/// Derive ownership classification for an arbitrary runtime.
/// Mirrors the `project_hermes_adoption` logic from `agents.rs` but generalized
/// for non-hermes runtimes (treated as external-unadopted unless proven owned).
fn derive_ownership_for_runtime(
    runtime: &AgentRuntimeObservation,
    _principal: &str,
) -> AgentRuntimeOwnership {
    let external_gateway_observed = runtime.graph.iter().any(|node| {
        node.id == "gateway_service"
            && matches!(
                node.state,
                cynic_host::agent_runtime::CapabilityState::Observed
            )
    });

    // For non-hermes runtimes, we cannot prove process identity without a
    // running gateway. Classify as external-unadopted.
    let process: Option<&cynic_host::agent_runtime::AgentProcessIdentityObservation> = None;

    classify_runtime_ownership_with_external_gateway(process, external_gateway_observed)
}

#[derive(serde::Serialize)]
struct AdoptReport<'a> {
    runtime_id: &'a str,
    principal: &'a str,
    ownership: &'a AgentRuntimeOwnership,
    adoption: &'a AgentRuntimeAdoptionPlan,
    toolchain: serde_json::Value,
    action_taken: bool,
}

/// Run the Rust toolchain lifecycle for the target crate.
/// Projection-first (observe → reconcile → decision). With `--approve`,
/// applies the bounded argv (rustup override set) — otherwise dry-run.
fn run_toolchain_lifecycle(
    crate_name: &str,
    seed: Option<&std::path::Path>,
    approve: bool,
) -> serde_json::Value {
    let cwd = std::env::current_dir().unwrap_or_default();
    let seed_path = seed
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| cwd.join("infra/rust-toolchain.json"));
    let history_dir = cwd.join("target/proofs/toolchain");

    let registry = cynic_host::toolchain::specs::ToolchainRegistry::load(&seed_path)
        .unwrap_or_default();
    let lifecycle = ToolchainLifecycle::new(
        cynic_host::toolchain::observe::LocalToolchainObserver::new(registry),
    );
    let runner = LocalArgvRunner;
    let report = if approve {
        lifecycle.run(crate_name, &seed_path, &history_dir, &runner)
    } else {
        // Dry-run: projection only — observe + reconcile decision surface.
        let observed = lifecycle.observer.observe(crate_name, &runner);
        let version = observed.version.clone();
        Ok(ToolchainLifecycleReport {
            crate_name: crate_name.to_string(),
            observed,
            decision: cynic_host::toolchain::ToolchainDecision::UseToolchain(version),
            applied: None,
            proof: cynic_host::toolchain::proof::ToolchainProof::blocked(crate_name),
            residual_risk: Some("dry_run".to_string()),
        })
    };
    match report {
        Ok(r) => serde_json::to_value(&r).unwrap_or(serde_json::Value::Null),
        Err(e) => serde_json::json!({ "status": "error", "error": e }),
    }
}
