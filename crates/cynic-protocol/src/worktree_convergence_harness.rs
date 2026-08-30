//! Laboratory test harness for multi-agent Git convergence and reconciliation strategies.
//!
//! Evaluates competing engineering methods under simulated LLM agent chaos (stale divergence,
//! lexical collisions, silent semantic breaks, zombie abandonment, concurrent bursts)
//! and quantitatively measures vital operational KPIs without requiring live disk I/O.

use serde::{Deserialize, Serialize};

pub const WORKTREE_CONVERGENCE_HARNESS_SCHEMA: &str = "cynic.worktree_convergence.harness.v1";

/// Competing engineering strategies for multi-agent Git convergence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConvergenceStrategy {
    /// Naive sequential FIFO rebase and merge; aborts on any conflict.
    OptimisticSerialRebase,
    /// Speculative multi-branch batch tree merge with bisection on failure.
    SpeculativeBatching,
    /// CYNIC Hierarchical Fenced Reconciler with Scratch Validator & SHA re-proofing.
    HierarchicalFencedReconciler,
    /// AST-aware semantic reconciliation with automated structural AST fixes.
    AstSemanticAssisted,
}

/// Nature of modification and conflict potential in simulated agent work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentWorkloadKind {
    /// Modifies disjoint files/crates (zero conflict potential).
    OrthogonalSlice { crate_name: String },
    /// Modifies same file on non-overlapping line ranges (clean lexical rebase).
    SharedFileNonConflicting { file_path: String },
    /// Touches lockfile or module manifest (Cargo.lock / mod.rs trivial conflict).
    TrivialManifestConflict { manifest_path: String },
    /// Modifies a shared signature: zero git text conflict, but breaks caller after rebase.
    SilentSemanticBreak {
        producer_crate: String,
        consumer_crate: String,
        broken_symbol: String,
    },
    /// Agent crashes mid-flight and abandons worktree (heartbeat lost).
    ZombieAbandonment { idle_time_seconds: u64 },
    /// Stale fork: agent branched N commits behind current main.
    StaleForkDivergence { commits_behind: u32 },
}

/// Simulated agent work submitted to the convergence engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAgentSubmission {
    pub agent_id: String,
    pub branch_name: String,
    pub fork_commit_sha: String,
    pub branch_head_sha: String,
    pub workload: AgentWorkloadKind,
    pub initial_tests_pass: bool,
    pub lease_id: String,
    pub lease_ttl_seconds: u64,
    pub heartbeat_age_seconds: u64,
}

/// Laboratory scenario containing initial repository state and concurrent agent submissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceLabScenario {
    pub scenario_id: String,
    pub description: String,
    pub main_head_sha: String,
    pub main_commit_height: u64,
    pub concurrent_submissions: Vec<SimulatedAgentSubmission>,
}

/// Quantitative KPIs measured by the laboratory harness.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConvergenceHarnessMetrics {
    /// Number of slices successfully converged to main.
    pub successful_merges: u32,
    /// Number of slices quarantined due to semantic breaks or hard conflicts.
    pub quarantined_count: u32,
    /// Number of zombie/abandoned branches pruned.
    pub pruned_zombies: u32,
    /// Number of cases escalated to human sovereign HITL.
    pub human_escalations: u32,
    /// Ratio of main commits that maintain 100% green tests (must be 1.0 for zero corruption).
    pub zero_corruption_rate: f64,
    /// Simulated token/compute cost (arbitrary units: compilation rounds + token dissipation).
    pub compute_cost_units: u64,
    /// Effective throughput: merges per simulated round.
    pub convergence_efficiency: f64,
}

/// Complete benchmark result for one strategy evaluated on a lab scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergenceLabReport {
    pub schema: String,
    pub scenario_id: String,
    pub strategy: ConvergenceStrategy,
    pub metrics: ConvergenceHarnessMetrics,
    pub decisions: Vec<SimulatedReconcileDecision>,
    pub conclusion: String,
}

/// Discrete decision produced for each submitted agent branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedReconcileDecision {
    pub agent_id: String,
    pub branch_name: String,
    pub action: SimulatedReconcileAction,
    pub resulting_main_sha: Option<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SimulatedReconcileAction {
    FastForwardMerged,
    RebasedAndMerged,
    QuarantinedForSemanticBreak,
    PrunedAsZombie,
    AutoResolvedManifestAndMerged,
    EscalatedToHuman,
    RejectedStaleDrift,
}

/// Pure evaluation function executing a laboratory benchmark for a given strategy.
pub fn evaluate_convergence_strategy(
    scenario: &ConvergenceLabScenario,
    strategy: ConvergenceStrategy,
) -> ConvergenceLabReport {
    let mut successful_merges = 0;
    let mut quarantined_count = 0;
    let mut pruned_zombies = 0;
    let mut human_escalations = 0;
    let mut corrupted_main_count = 0;
    let mut compute_cost = 0;
    let mut decisions = Vec::new();

    let mut current_main_sha = scenario.main_head_sha.clone();
    let mut current_main_height = scenario.main_commit_height;

    // Track active semantic breaks on main
    let mut broken_symbols_on_main = std::collections::HashSet::new();

    for sub in &scenario.concurrent_submissions {
        compute_cost += 10; // Base observation cost

        // 1. Check for zombie abandonment
        if sub.heartbeat_age_seconds > sub.lease_ttl_seconds {
            pruned_zombies += 1;
            decisions.push(SimulatedReconcileDecision {
                agent_id: sub.agent_id.clone(),
                branch_name: sub.branch_name.clone(),
                action: SimulatedReconcileAction::PrunedAsZombie,
                resulting_main_sha: None,
                rationale: format!(
                    "Heartbeat age ({}s) exceeded lease TTL ({}s)",
                    sub.heartbeat_age_seconds, sub.lease_ttl_seconds
                ),
            });
            continue;
        }

        // 2. Check for excessive stale divergence (> 30 commits behind)
        if let AgentWorkloadKind::StaleForkDivergence { commits_behind } = sub.workload {
            if commits_behind > 30 {
                decisions.push(SimulatedReconcileDecision {
                    agent_id: sub.agent_id.clone(),
                    branch_name: sub.branch_name.clone(),
                    action: SimulatedReconcileAction::RejectedStaleDrift,
                    resulting_main_sha: None,
                    rationale: format!(
                        "Branch is {} commits behind main (> 30 limit)",
                        commits_behind
                    ),
                });
                continue;
            }
        }

        // 3. Strategy-specific resolution logic
        match strategy {
            ConvergenceStrategy::OptimisticSerialRebase => {
                compute_cost += 20; // Rebase test cost
                match &sub.workload {
                    AgentWorkloadKind::OrthogonalSlice { .. } => {
                        current_main_height += 1;
                        current_main_sha = format!("sha-{}-{}", sub.agent_id, current_main_height);
                        successful_merges += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::FastForwardMerged,
                            resulting_main_sha: Some(current_main_sha.clone()),
                            rationale: "Orthogonal change merged clean".to_string(),
                        });
                    }
                    AgentWorkloadKind::SilentSemanticBreak {
                        consumer_crate,
                        broken_symbol,
                        ..
                    } => {
                        // Naive rebase fails to detect semantic break if text merges cleanly!
                        // This corrupts main!
                        if broken_symbols_on_main.contains(broken_symbol) {
                            corrupted_main_count += 1;
                            current_main_height += 1;
                            current_main_sha =
                                format!("sha-CORRUPT-{}-{}", sub.agent_id, current_main_height);
                            successful_merges += 1;
                            decisions.push(SimulatedReconcileDecision {
                                agent_id: sub.agent_id.clone(),
                                branch_name: sub.branch_name.clone(),
                                action: SimulatedReconcileAction::RebasedAndMerged,
                                resulting_main_sha: Some(current_main_sha.clone()),
                                rationale: format!(
                                    "Merged blindly despite semantic break in {}",
                                    consumer_crate
                                ),
                            });
                        } else {
                            broken_symbols_on_main.insert(broken_symbol.clone());
                            current_main_height += 1;
                            current_main_sha =
                                format!("sha-{}-{}", sub.agent_id, current_main_height);
                            successful_merges += 1;
                            decisions.push(SimulatedReconcileDecision {
                                agent_id: sub.agent_id.clone(),
                                branch_name: sub.branch_name.clone(),
                                action: SimulatedReconcileAction::FastForwardMerged,
                                resulting_main_sha: Some(current_main_sha.clone()),
                                rationale: "Producer signature change merged".to_string(),
                            });
                        }
                    }
                    AgentWorkloadKind::TrivialManifestConflict { .. } => {
                        // Naive rebase escalates all textual conflicts to human
                        human_escalations += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::EscalatedToHuman,
                            resulting_main_sha: None,
                            rationale: "Text conflict in manifest aborted rebase".to_string(),
                        });
                    }
                    _ => {
                        current_main_height += 1;
                        current_main_sha = format!("sha-{}-{}", sub.agent_id, current_main_height);
                        successful_merges += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::RebasedAndMerged,
                            resulting_main_sha: Some(current_main_sha.clone()),
                            rationale: "Clean rebase merged".to_string(),
                        });
                    }
                }
            }

            ConvergenceStrategy::HierarchicalFencedReconciler => {
                compute_cost += 30; // Scratch validator + remote proof validation
                match &sub.workload {
                    AgentWorkloadKind::OrthogonalSlice { .. } => {
                        current_main_height += 1;
                        current_main_sha = format!("sha-{}-{}", sub.agent_id, current_main_height);
                        successful_merges += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::FastForwardMerged,
                            resulting_main_sha: Some(current_main_sha.clone()),
                            rationale: "Orthogonal slice passed scratch validator".to_string(),
                        });
                    }
                    AgentWorkloadKind::SilentSemanticBreak {
                        consumer_crate,
                        broken_symbol,
                        ..
                    } => {
                        if broken_symbols_on_main.contains(broken_symbol) {
                            // Scratch validator re-checks compilation on result SHA -> catches the break!
                            quarantined_count += 1;
                            decisions.push(SimulatedReconcileDecision {
                                agent_id: sub.agent_id.clone(),
                                branch_name: sub.branch_name.clone(),
                                action: SimulatedReconcileAction::QuarantinedForSemanticBreak,
                                resulting_main_sha: None,
                                rationale: format!(
                                    "Scratch validator detected broken symbol '{}' in consumer crate '{}'",
                                    broken_symbol, consumer_crate
                                ),
                            });
                        } else {
                            broken_symbols_on_main.insert(broken_symbol.clone());
                            current_main_height += 1;
                            current_main_sha =
                                format!("sha-{}-{}", sub.agent_id, current_main_height);
                            successful_merges += 1;
                            decisions.push(SimulatedReconcileDecision {
                                agent_id: sub.agent_id.clone(),
                                branch_name: sub.branch_name.clone(),
                                action: SimulatedReconcileAction::FastForwardMerged,
                                resulting_main_sha: Some(current_main_sha.clone()),
                                rationale: "Producer signature updated with verified proof"
                                    .to_string(),
                            });
                        }
                    }
                    AgentWorkloadKind::TrivialManifestConflict { manifest_path } => {
                        // Deterministic lockfile/manifest auto-resolution in scratch validator
                        compute_cost += 15;
                        current_main_height += 1;
                        current_main_sha =
                            format!("sha-AUTORES-{}-{}", sub.agent_id, current_main_height);
                        successful_merges += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::AutoResolvedManifestAndMerged,
                            resulting_main_sha: Some(current_main_sha.clone()),
                            rationale: format!(
                                "Deterministically regenerated manifest at '{}'",
                                manifest_path
                            ),
                        });
                    }
                    _ => {
                        current_main_height += 1;
                        current_main_sha = format!("sha-{}-{}", sub.agent_id, current_main_height);
                        successful_merges += 1;
                        decisions.push(SimulatedReconcileDecision {
                            agent_id: sub.agent_id.clone(),
                            branch_name: sub.branch_name.clone(),
                            action: SimulatedReconcileAction::RebasedAndMerged,
                            resulting_main_sha: Some(current_main_sha.clone()),
                            rationale: "Rebased and verified on scratch SHA before merge"
                                .to_string(),
                        });
                    }
                }
            }

            ConvergenceStrategy::SpeculativeBatching => {
                compute_cost += 45; // Batch tree compilation
                current_main_height += 1;
                current_main_sha = format!("sha-BATCH-{}", current_main_height);
                successful_merges += 1;
                decisions.push(SimulatedReconcileDecision {
                    agent_id: sub.agent_id.clone(),
                    branch_name: sub.branch_name.clone(),
                    action: SimulatedReconcileAction::RebasedAndMerged,
                    resulting_main_sha: Some(current_main_sha.clone()),
                    rationale: "Merged in speculative batch".to_string(),
                });
            }

            ConvergenceStrategy::AstSemanticAssisted => {
                compute_cost += 60; // AST parse & rewrite
                current_main_height += 1;
                current_main_sha = format!("sha-AST-{}", current_main_height);
                successful_merges += 1;
                decisions.push(SimulatedReconcileDecision {
                    agent_id: sub.agent_id.clone(),
                    branch_name: sub.branch_name.clone(),
                    action: SimulatedReconcileAction::RebasedAndMerged,
                    resulting_main_sha: Some(current_main_sha.clone()),
                    rationale: "AST codemod reconciled symbols".to_string(),
                });
            }
        }
    }

    let total_merges = successful_merges + corrupted_main_count;
    let zero_corruption_rate = if total_merges == 0 {
        1.0
    } else {
        (successful_merges as f64) / ((successful_merges + corrupted_main_count) as f64)
    };

    let total_submissions = scenario.concurrent_submissions.len() as f64;
    let convergence_efficiency = if total_submissions == 0.0 {
        0.0
    } else {
        (successful_merges as f64) / total_submissions
    };

    let metrics = ConvergenceHarnessMetrics {
        successful_merges,
        quarantined_count,
        pruned_zombies,
        human_escalations,
        zero_corruption_rate,
        compute_cost_units: compute_cost,
        convergence_efficiency,
    };

    let conclusion = format!(
        "Strategy {:?}: {} merged, {} quarantined, {} zombies pruned, {} escalations. Zero-Corruption: {:.1}%, Compute Cost: {} units",
        strategy,
        metrics.successful_merges,
        metrics.quarantined_count,
        metrics.pruned_zombies,
        metrics.human_escalations,
        metrics.zero_corruption_rate * 100.0,
        metrics.compute_cost_units
    );

    ConvergenceLabReport {
        schema: WORKTREE_CONVERGENCE_HARNESS_SCHEMA.to_string(),
        scenario_id: scenario.scenario_id.clone(),
        strategy,
        metrics,
        decisions,
        conclusion,
    }
}

/// Builds canonical chaotic multi-agent laboratory scenarios for benchmark evaluations.
pub fn canonical_convergence_lab_scenarios() -> Vec<ConvergenceLabScenario> {
    vec![
        ConvergenceLabScenario {
            scenario_id: "agent_chaos_burst_6x".to_string(),
            description: "6 concurrent agents with mixed orthogonal, manifest collision, semantic break, and zombie states".to_string(),
            main_head_sha: "main-0000".to_string(),
            main_commit_height: 100,
            concurrent_submissions: vec![
                SimulatedAgentSubmission {
                    agent_id: "hermes-01".to_string(),
                    branch_name: "agent/hermes/host-metrics".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-hermes-01".to_string(),
                    workload: AgentWorkloadKind::OrthogonalSlice { crate_name: "cynic-host".to_string() },
                    initial_tests_pass: true,
                    lease_id: "lease-01".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 120,
                },
                SimulatedAgentSubmission {
                    agent_id: "codex-02".to_string(),
                    branch_name: "agent/codex/inference-pool".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-codex-02".to_string(),
                    workload: AgentWorkloadKind::SilentSemanticBreak {
                        producer_crate: "cynic-inference".to_string(),
                        consumer_crate: "cynic-kernel".to_string(),
                        broken_symbol: "BackendHealthPort::probe".to_string(),
                    },
                    initial_tests_pass: true,
                    lease_id: "lease-02".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 300,
                },
                SimulatedAgentSubmission {
                    agent_id: "talaria-03".to_string(),
                    branch_name: "agent/talaria/kernel-caller".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-talaria-03".to_string(),
                    workload: AgentWorkloadKind::SilentSemanticBreak {
                        producer_crate: "cynic-inference".to_string(),
                        consumer_crate: "cynic-kernel".to_string(),
                        broken_symbol: "BackendHealthPort::probe".to_string(),
                    },
                    initial_tests_pass: true,
                    lease_id: "lease-03".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 150,
                },
                SimulatedAgentSubmission {
                    agent_id: "omp-04".to_string(),
                    branch_name: "agent/omp/manifest-add".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-omp-04".to_string(),
                    workload: AgentWorkloadKind::TrivialManifestConflict {
                        manifest_path: "Cargo.lock".to_string(),
                    },
                    initial_tests_pass: true,
                    lease_id: "lease-04".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 60,
                },
                SimulatedAgentSubmission {
                    agent_id: "zombie-05".to_string(),
                    branch_name: "agent/zombie/crashed-work".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-zombie-05".to_string(),
                    workload: AgentWorkloadKind::ZombieAbandonment { idle_time_seconds: 4000 },
                    initial_tests_pass: true,
                    lease_id: "lease-05".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 4000,
                },
                SimulatedAgentSubmission {
                    agent_id: "drift-06".to_string(),
                    branch_name: "agent/drift/ancient-work".to_string(),
                    fork_commit_sha: "main-0000".to_string(),
                    branch_head_sha: "head-drift-06".to_string(),
                    workload: AgentWorkloadKind::StaleForkDivergence { commits_behind: 45 },
                    initial_tests_pass: true,
                    lease_id: "lease-06".to_string(),
                    lease_ttl_seconds: 1800,
                    heartbeat_age_seconds: 200,
                },
            ],
        }
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_hierarchical_reconciler_prevents_main_corruption() {
        let scenarios = canonical_convergence_lab_scenarios();
        let scenario = &scenarios[0];

        // 1. Test Naive Optimistic Serial Rebase: Corrupts main on silent semantic break!
        let naive_report =
            evaluate_convergence_strategy(scenario, ConvergenceStrategy::OptimisticSerialRebase);
        assert!(
            naive_report.metrics.zero_corruption_rate < 1.0,
            "Naive rebase must fail zero-corruption"
        );
        assert!(
            naive_report.metrics.human_escalations > 0,
            "Naive rebase unnecessarily escalates trivial lock conflicts"
        );

        // 2. Test CYNIC Hierarchical Fenced Reconciler: 100% Zero-Corruption + Auto-Resolves lockfiles + Quarantines semantic break!
        let cynic_report = evaluate_convergence_strategy(
            scenario,
            ConvergenceStrategy::HierarchicalFencedReconciler,
        );
        assert_eq!(
            cynic_report.metrics.zero_corruption_rate, 1.0,
            "CYNIC reconciler must maintain 100% zero corruption"
        );
        assert_eq!(
            cynic_report.metrics.quarantined_count, 1,
            "Must quarantine silent semantic break"
        );
        assert_eq!(
            cynic_report.metrics.pruned_zombies, 1,
            "Must prune zombie abandonment"
        );
        assert_eq!(
            cynic_report.metrics.human_escalations, 0,
            "Zero human escalations for nominal/trivial cases"
        );
    }
}
