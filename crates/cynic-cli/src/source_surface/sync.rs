//! Source layer status, synchronization and publication execution.

use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use crate::source_surface::sync_coherence;

pub(crate) fn execute_status(root: &Path, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let decision = cynic_host::source_sync::reconcile(&observation);
    if json {
        emit_pretty_json(&decision)
    } else {
        println!("Source sync actions: {}", decision.actions.len());
        println!("Needs review: {}", decision.needs_review);
        if !decision.unknowns.is_empty() {
            println!("Unknowns: {:?}", decision.unknowns);
        }
        ExitCode::SUCCESS
    }
}

pub(crate) fn execute_sync(root: &Path, apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let decision = cynic_host::source_sync::reconcile(&observation);
    let plan = cynic_host::source_sync::plan(&decision);
    if apply {
        let admission =
            cynic_host::adapters::source_admission_adapter::admit_source_sync(root, &decision);
        if !admission.admitted {
            let coherence =
                sync_coherence::project_sync_coherence_vector(&observation, &decision, None);
            let report = sync_coherence::SourceSyncReport {
                schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
                coherence,
                plan: Some(plan),
                proof: None,
            };
            if json {
                emit_pretty_json(&report)
            } else {
                println!("Source sync BLOCKED by admission gate.");
                println!("Status:   {:?}", report.coherence.status);
                println!(
                    "Compass:  {}",
                    report.coherence.compass.desired_state_digest
                );
                println!("Blockers: {:?}", report.coherence.blockers);
                ExitCode::FAILURE
            }
        } else {
            let proof = cynic_host::source_sync::execute_plan(root, &plan);
            let read_back = if proof.success {
                observation.cortex.head_sha.as_deref()
            } else {
                None
            };
            let coherence =
                sync_coherence::project_sync_coherence_vector(&observation, &decision, read_back);
            let report = sync_coherence::SourceSyncReport {
                schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
                coherence,
                plan: Some(plan),
                proof: Some(proof.clone()),
            };
            if json {
                emit_pretty_json(&report)
            } else {
                println!(
                    "Sync execution: {} steps, success={}",
                    proof.steps_completed.len(),
                    proof.success
                );
                println!("Status:   {:?}", report.coherence.status);
                println!(
                    "Compass:  {}",
                    report.coherence.compass.desired_state_digest
                );
                for result in &proof.steps_completed {
                    let status_sym = if result.success { "PASS" } else { "FAIL" };
                    println!("  [{status_sym}] {:?}", result.step);
                }
                if proof.success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
    } else {
        let coherence =
            sync_coherence::project_sync_coherence_vector(&observation, &decision, None);
        let report = sync_coherence::SourceSyncReport {
            schema: sync_coherence::SOURCE_SYNC_REPORT_SCHEMA,
            coherence,
            plan: Some(plan.clone()),
            proof: None,
        };
        if json {
            emit_pretty_json(&report)
        } else {
            println!("=== CYNIC Source Coherence Vector ===");
            println!("  Subject:    {}", report.coherence.compass.subject_id);
            println!("  Vertical:   {}", report.coherence.compass.vertical_id);
            println!("  Status:     {:?}", report.coherence.status);
            println!(
                "  Compass:    {}",
                report.coherence.compass.desired_state_digest
            );
            println!("  Decision:   {}", report.coherence.vector.decision);
            if let Some(act) = &report.coherence.vector.action {
                println!("  Action:     {}", act);
            }
            if !report.coherence.blockers.is_empty() {
                println!("  Blockers:   {:?}", report.coherence.blockers);
            }
            if !report.coherence.residual_unknowns.is_empty() {
                println!("  Unknowns:   {:?}", report.coherence.residual_unknowns);
            }
            println!(
                "\nSync plan: {} steps (dry-run, pass --apply to execute)",
                plan.steps.len()
            );
            ExitCode::SUCCESS
        }
    }
}

pub(crate) fn execute_publish(root: &Path, json: bool) -> ExitCode {
    let contract_path = root.join("contracts/publication.json");
    let raw = match std::fs::read_to_string(&contract_path) {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("blocked: cannot read {}: {e}", contract_path.display());
            return ExitCode::FAILURE;
        }
    };
    let contract = match cynic_vcs::PublicationContract::from_json(&raw) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("blocked: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runner = cynic_host::adapters::vcs_runner::LocalVcsRunner;
    let (tracked, files) = match cynic_vcs::source_worktree_cli::observe_publication_inputs(
        root, &contract, &runner,
    ) {
        Ok(inputs) => inputs,
        Err(e) => {
            eprintln!("blocked: {e}");
            return ExitCode::FAILURE;
        }
    };

    let plan = cynic_vcs::plan_publication(&contract, &tracked, &files);
    let admitted = plan.is_admitted();

    if json {
        emit_pretty_json(&plan);
        return if admitted {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    println!(
        "Publication plan — {} ({})",
        plan.contract_schema, plan.contract_status
    );
    println!("  tracked:   {}", plan.tracked_count);
    println!("  published: {}", plan.published_count);
    println!("  excluded:  {}", plan.excluded_count);
    println!("  undeclared: {}", plan.undeclared.len());
    println!("  dead rules: {}", plan.dead_rules.len());
    println!(
        "  uncovered private literals: {}",
        plan.uncovered_literals.len()
    );
    for literal in plan.uncovered_literals.iter().take(10) {
        println!("    {} ({} path(s))", literal.literal, literal.paths.len());
    }
    if admitted {
        println!("ADMITTED — the private tree satisfies contracts/publication.json.");
        ExitCode::SUCCESS
    } else {
        println!("REFUSED — {} blocker(s):", plan.blockers.len());
        for blocker in plan.blockers.iter().take(20) {
            println!("    {blocker}");
        }
        if plan.blockers.len() > 20 {
            println!("    … and {} more", plan.blockers.len() - 20);
        }
        ExitCode::FAILURE
    }
}
