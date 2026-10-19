//! Formal invariant proofs for compute workload placement and capacity predicates.
//!
//! Epistemic foundation: Mathematical invariants must hold for ANY randomized
//! or boundary input. No assumption, no implicit grant.

use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    ComputeRequirements, SubstrateKind, SubstrateObservation, decide_compute_placement,
};

#[test]
fn invariant_safety_admitted_placement_never_violates_physical_capacities() {
    // Generate boundary matrices of requirements and substrates
    let vcpu_scenarios = [1, 2, 4, 8, 16, 64];
    let ram_scenarios = [
        512 * 1024 * 1024,
        2 * 1024 * 1024 * 1024,
        8 * 1024 * 1024 * 1024,
        64 * 1024 * 1024 * 1024,
    ];
    let gpu_scenarios = [false, true];

    for &vcpus in &vcpu_scenarios {
        for &ram in &ram_scenarios {
            for &gpu in &gpu_scenarios {
                let reqs = ComputeRequirements {
                    vcpus,
                    memory_bytes: ram,
                    disk_bytes: 10 * 1024 * 1024 * 1024,
                    requires_gpu: gpu,
                    preferred_strategy: SandboxStrategy::BwrapEphemeral,
                    allowed_strategies: vec![
                        SandboxStrategy::BwrapEphemeral,
                        SandboxStrategy::LxcUnprivileged,
                    ],
                };

                let fleet = vec![
                    SubstrateObservation {
                        name: "sub-edge-small".to_string(),
                        kind: SubstrateKind::LocalBwrap,
                        available_vcpus: 2,
                        available_memory_bytes: 1024 * 1024 * 1024,
                        available_disk_bytes: 20 * 1024 * 1024 * 1024,
                        has_gpu: false,
                        supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
                    },
                    SubstrateObservation {
                        name: "sub-cluster-large".to_string(),
                        kind: SubstrateKind::ProxmoxLxc,
                        available_vcpus: 32,
                        available_memory_bytes: 32 * 1024 * 1024 * 1024,
                        available_disk_bytes: 500 * 1024 * 1024 * 1024,
                        has_gpu: true,
                        supported_strategies: vec![SandboxStrategy::LxcUnprivileged],
                    },
                ];

                let decision = decide_compute_placement(&reqs, &fleet);

                if decision.admitted {
                    let sub_name = decision
                        .selected_substrate
                        .as_ref()
                        .expect("admitted decision must name substrate");
                    let sub = fleet
                        .iter()
                        .find(|s| &s.name == sub_name)
                        .expect("selected substrate must exist in fleet");

                    // INVARIANT 1: Physical capacity bounds
                    assert!(
                        sub.available_vcpus >= reqs.vcpus,
                        "Capacity violation: allocated vcpu {} on substrate with {}",
                        reqs.vcpus,
                        sub.available_vcpus
                    );
                    assert!(
                        sub.available_memory_bytes >= reqs.memory_bytes,
                        "Capacity violation: allocated RAM {} on substrate with {}",
                        reqs.memory_bytes,
                        sub.available_memory_bytes
                    );
                    assert!(
                        sub.available_disk_bytes >= reqs.disk_bytes,
                        "Capacity violation: allocated disk {} on substrate with {}",
                        reqs.disk_bytes,
                        sub.available_disk_bytes
                    );

                    // INVARIANT 2: GPU requirement strictly enforced
                    if reqs.requires_gpu {
                        assert!(
                            sub.has_gpu,
                            "GPU violation: workload required GPU but substrate has none"
                        );
                    }

                    // INVARIANT 3: Strategy containment
                    let chosen_strategy = decision
                        .selected_strategy
                        .expect("admitted decision must specify strategy");
                    assert!(
                        reqs.allowed_strategies.contains(&chosen_strategy),
                        "Isolation violation: chosen strategy {:?} not allowed in {:?}",
                        chosen_strategy,
                        reqs.allowed_strategies
                    );
                    assert!(
                        sub.supported_strategies.contains(&chosen_strategy),
                        "Substrate compatibility violation: strategy {:?} not supported by {}",
                        chosen_strategy,
                        sub.name
                    );
                } else {
                    // INVARIANT 4: Explicit non-empty reason on rejection
                    assert!(
                        !decision.reason.is_empty(),
                        "Rejection must contain explicit evidence reason"
                    );
                    assert!(
                        decision.selected_substrate.is_none(),
                        "Rejected decision must have no substrate"
                    );
                    assert!(
                        decision.selected_strategy.is_none(),
                        "Rejected decision must have no strategy"
                    );
                }
            }
        }
    }
}

#[test]
fn invariant_fail_closed_on_empty_fleet() {
    let reqs = ComputeRequirements::default();
    let empty_fleet: Vec<SubstrateObservation> = vec![];
    let decision = decide_compute_placement(&reqs, &empty_fleet);

    assert!(!decision.admitted);
    assert_eq!(decision.reason, "no compute substrates available in fleet");
    assert!(decision.selected_substrate.is_none());
}

#[test]
fn invariant_total_determinism() {
    let reqs = ComputeRequirements::default();
    let fleet = vec![SubstrateObservation {
        name: "deterministic-node".to_string(),
        kind: SubstrateKind::LocalBwrap,
        available_vcpus: 4,
        available_memory_bytes: 4 * 1024 * 1024 * 1024,
        available_disk_bytes: 20 * 1024 * 1024 * 1024,
        has_gpu: false,
        supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
    }];

    let run_1 = decide_compute_placement(&reqs, &fleet);
    let run_2 = decide_compute_placement(&reqs, &fleet);

    assert_eq!(run_1, run_2, "Placement decision must be 100% deterministic");
}
