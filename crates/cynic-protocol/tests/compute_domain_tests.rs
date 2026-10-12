//! Verification of compute requirements, placement predicates, and substrate matching.

use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    ComputeRequirements, SubstrateKind, SubstrateObservation, decide_compute_placement,
};

#[test]
fn placement_selects_preferred_bwrap_strategy_on_local_host() {
    let reqs = ComputeRequirements {
        vcpus: 2,
        memory_bytes: 2 * 1024 * 1024 * 1024,
        disk_bytes: 10 * 1024 * 1024 * 1024,
        requires_gpu: false,
        preferred_strategy: SandboxStrategy::BwrapEphemeral,
        allowed_strategies: vec![
            SandboxStrategy::BwrapEphemeral,
            SandboxStrategy::LxcUnprivileged,
        ],
    };

    let substrates = vec![
        SubstrateObservation {
            name: "cortex-local".to_string(),
            kind: SubstrateKind::LocalBwrap,
            available_vcpus: 16,
            available_memory_bytes: 32 * 1024 * 1024 * 1024,
            available_disk_bytes: 500 * 1024 * 1024 * 1024,
            has_gpu: false,
            supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
        },
        SubstrateObservation {
            name: "proxmox-pve".to_string(),
            kind: SubstrateKind::ProxmoxLxc,
            available_vcpus: 32,
            available_memory_bytes: 64 * 1024 * 1024 * 1024,
            available_disk_bytes: 1000 * 1024 * 1024 * 1024,
            has_gpu: true,
            supported_strategies: vec![SandboxStrategy::LxcUnprivileged],
        },
    ];

    let decision = decide_compute_placement(&reqs, &substrates);
    assert!(decision.admitted);
    assert_eq!(decision.selected_substrate.as_deref(), Some("cortex-local"));
    assert_eq!(
        decision.selected_strategy,
        Some(SandboxStrategy::BwrapEphemeral)
    );
}

#[test]
fn placement_falls_back_to_lxc_on_proxmox_when_local_has_insufficient_ram() {
    let reqs = ComputeRequirements {
        vcpus: 4,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        disk_bytes: 50 * 1024 * 1024 * 1024,
        requires_gpu: false,
        preferred_strategy: SandboxStrategy::BwrapEphemeral,
        allowed_strategies: vec![
            SandboxStrategy::BwrapEphemeral,
            SandboxStrategy::LxcUnprivileged,
        ],
    };

    let substrates = vec![
        SubstrateObservation {
            name: "cortex-local".to_string(),
            kind: SubstrateKind::LocalBwrap,
            available_vcpus: 4,
            available_memory_bytes: 8 * 1024 * 1024 * 1024, // Insufficient (8GB < 16GB)
            available_disk_bytes: 500 * 1024 * 1024 * 1024,
            has_gpu: false,
            supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
        },
        SubstrateObservation {
            name: "proxmox-pve".to_string(),
            kind: SubstrateKind::ProxmoxLxc,
            available_vcpus: 32,
            available_memory_bytes: 64 * 1024 * 1024 * 1024, // Sufficient
            available_disk_bytes: 1000 * 1024 * 1024 * 1024,
            has_gpu: false,
            supported_strategies: vec![SandboxStrategy::LxcUnprivileged],
        },
    ];

    let decision = decide_compute_placement(&reqs, &substrates);
    assert!(decision.admitted);
    assert_eq!(decision.selected_substrate.as_deref(), Some("proxmox-pve"));
    assert_eq!(
        decision.selected_strategy,
        Some(SandboxStrategy::LxcUnprivileged)
    );
}
