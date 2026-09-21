//! End-to-end integration test for the CYNIC Trust-Chain vertical.
//!
//! Proves all 4 cryptographic proof layers in a federated multi-node environment:
//! 1. Phase 1 (Identity): Keygen -> CSR -> Multi-Root CA Issue -> Verify -> Disk Install.
//! 2. Phase 2 (Visibility): Authenticated Mesh Gossip -> Multi-Root PeerValidator -> Tamper Rejection -> Authenticated Fleet Health.
//! 3. Phase 3 (Authority): Signed Control Plan Dispatch -> Generation Monotonicity -> PlanVerifier Admission.
//! 4. Phase 4 (Sovereignty): Cryptographic Lease -> Monotonic Fencing Tokens -> Instant Authority Drop on Expiry -> Auditable Takeover History.

use std::sync::Arc;

use cynic_host::crypto::{CryptoProvider, Ed25519Provider};
use cynic_host::lease_manager::LeaseManager;
use cynic_host::node_identity_workflow::{NodeIdentityMaterial, NodeIdentityWorkflow};
use cynic_host::peer_validator::PeerValidator;
use cynic_host::plan_verifier::PlanVerifier;
use cynic_protocol::RuntimeSeedRequest;
use cynic_protocol::fleet_discovery::FleetNodeStatus;
use cynic_protocol::fleet_heartbeat::{
    FleetHeartbeat, HeartbeatValidationResult, aggregate_authenticated_fleet_health,
};
use cynic_protocol::node_control::{
    ControlPlanVerificationStatus, NODE_DESIRED_STATE_SCHEMA, NodeDesiredState,
};
use tempfile::tempdir;

#[test]
fn trust_chain_full_vertical_e2e() {
    let dir = tempdir().expect("tempdir");
    let provider = Arc::new(Ed25519Provider);
    let workflow = NodeIdentityWorkflow::new(provider.clone());

    let now_s = 1_700_000_000u64;
    let now_ms = now_s * 1000;

    // =========================================================================
    // Phase 1: Root of Trust & Multi-Root Identity Enrollment (Zero-SPOF)
    // =========================================================================
    // Primary Kernel CA and Standby Kernel CA (Dual-Root Federation)
    let (kernel_primary_secret, kernel_primary_public) =
        provider.generate_keypair().expect("kernel primary");
    let (kernel_standby_secret, kernel_standby_public) =
        provider.generate_keypair().expect("kernel standby");

    // Node 1 (Cortex) key generation and CSR
    let node1_key_path = dir.path().join("node1.key");
    let node1_cert_path = dir.path().join("node1.cert");
    let node1_material = workflow
        .generate_keypair(&node1_key_path)
        .expect("node 1 keypair");
    let node1_csr = workflow
        .create_csr("node-cortex", "cortex", &node1_material, now_s)
        .expect("node 1 csr");
    assert!(
        workflow
            .verify_csr(&node1_csr)
            .expect("node 1 csr verification")
    );

    // Node 2 (Worker) key generation and CSR
    let node2_key_path = dir.path().join("node2.key");
    let node2_cert_path = dir.path().join("node2.cert");
    let node2_material = workflow
        .generate_keypair(&node2_key_path)
        .expect("node 2 keypair");
    let node2_csr = workflow
        .create_csr("node-worker", "builder", &node2_material, now_s)
        .expect("node 2 csr");
    assert!(
        workflow
            .verify_csr(&node2_csr)
            .expect("node 2 csr verification")
    );

    // Node 1 is enrolled by Primary Kernel; Node 2 is enrolled by Standby Kernel
    let cert1 = workflow
        .issue_certificate(
            &node1_csr,
            "kernel-primary",
            &kernel_primary_secret,
            86400,
            now_s,
        )
        .expect("issue cert 1");
    let cert2 = workflow
        .issue_certificate(
            &node2_csr,
            "kernel-standby",
            &kernel_standby_secret,
            86400,
            now_s,
        )
        .expect("issue cert 2");

    // Install certificates locally
    workflow
        .install_certificate(&cert1, &node1_cert_path)
        .expect("install cert 1");
    workflow
        .install_certificate(&cert2, &node2_cert_path)
        .expect("install cert 2");

    // Verify certificates on disk
    let reloaded1 = workflow
        .load_certificate(&node1_cert_path)
        .expect("load cert 1");
    assert!(
        workflow
            .verify_certificate(&reloaded1, &kernel_primary_public, now_s)
            .expect("verify cert 1")
    );
    let reloaded2 = workflow
        .load_certificate(&node2_cert_path)
        .expect("load cert 2");
    assert!(
        workflow
            .verify_certificate(&reloaded2, &kernel_standby_public, now_s)
            .expect("verify cert 2")
    );

    // =========================================================================
    // Phase 2: Authenticated Mesh Gossip & Local PeerValidator (Zero-SPOF)
    // =========================================================================
    let mut peer_validator = PeerValidator::new(provider.clone());
    peer_validator
        .register_root_authority("kernel-primary", &kernel_primary_public)
        .expect("reg primary");
    peer_validator
        .register_root_authority("kernel-standby", &kernel_standby_public)
        .expect("reg standby");

    // Nodes register peer certificates into local cache using multi-root resolution
    peer_validator
        .register_peer_certificate_with_roots(cert1.clone(), now_s)
        .expect("register cert 1");
    peer_validator
        .register_peer_certificate_with_roots(cert2.clone(), now_s)
        .expect("register cert 2");

    // 1. Valid signed heartbeat from Node 1
    let hb1 = FleetHeartbeat {
        node_id: "node-cortex".to_string(),
        emitted_at_unix_s: now_s + 5,
        self_status: FleetNodeStatus::Alive,
        uptime_s: 100,
        unknowns: vec![],
    };
    let signed1 = peer_validator
        .sign_heartbeat(&hb1, &node1_material.private_key, &cert1.certificate_id)
        .expect("sign hb 1");
    assert_eq!(
        peer_validator.validate_heartbeat(&signed1, now_s + 10, 30),
        HeartbeatValidationResult::Valid
    );

    // 2. Valid signed heartbeat from Node 2
    let hb2 = FleetHeartbeat {
        node_id: "node-worker".to_string(),
        emitted_at_unix_s: now_s + 5,
        self_status: FleetNodeStatus::Alive,
        uptime_s: 50,
        unknowns: vec![],
    };
    let signed2 = peer_validator
        .sign_heartbeat(&hb2, &node2_material.private_key, &cert2.certificate_id)
        .expect("sign hb 2");
    assert_eq!(
        peer_validator.validate_heartbeat(&signed2, now_s + 10, 30),
        HeartbeatValidationResult::Valid
    );

    // 3. Pirate node (unregistered) tries to inject gossip -> Rejected as UntrustedSigner
    let (pirate_secret, _) = provider.generate_keypair().expect("pirate keypair");
    let hb_pirate = FleetHeartbeat {
        node_id: "node-pirate".to_string(),
        emitted_at_unix_s: now_s + 5,
        self_status: FleetNodeStatus::Alive,
        uptime_s: 10,
        unknowns: vec![],
    };
    let signed_pirate = peer_validator
        .sign_heartbeat(
            &hb_pirate,
            &pirate_secret,
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        )
        .expect("sign pirate");
    assert_eq!(
        peer_validator.validate_heartbeat(&signed_pirate, now_s + 10, 30),
        HeartbeatValidationResult::UntrustedSigner
    );

    // 4. Tampered heartbeat -> Rejected as InvalidSignature
    let mut tampered_hb = signed2.clone();
    tampered_hb.heartbeat.self_status = FleetNodeStatus::Degraded;
    assert_eq!(
        peer_validator.validate_heartbeat(&tampered_hb, now_s + 10, 30),
        HeartbeatValidationResult::InvalidSignature
    );

    // 5. Authenticated health aggregation computes health from strictly verified pulses
    let declared_nodes = vec!["node-cortex".to_string(), "node-worker".to_string()];
    let verified_pulses = vec![hb1.clone(), hb2.clone()];
    let rejected_pulses = vec![(
        "node-pirate".to_string(),
        HeartbeatValidationResult::UntrustedSigner,
    )];

    let fleet_health = aggregate_authenticated_fleet_health(
        &declared_nodes,
        &verified_pulses,
        &rejected_pulses,
        now_s + 10,
        30,
    );
    assert_eq!(fleet_health.total_declared, 2);
    assert_eq!(fleet_health.total_verified, 2);
    assert_eq!(fleet_health.alive, 2);
    assert_eq!(fleet_health.untrusted, vec!["node-pirate".to_string()]);
    assert!(!fleet_health.partition_detected);

    // =========================================================================
    // Phase 3: Secure Control Plan & Admission Verification
    // =========================================================================
    let mut plan_verifier = PlanVerifier::new(provider.clone());
    plan_verifier
        .register_kernel_authority("kernel-primary", &kernel_primary_public)
        .expect("reg kernel");

    let desired_state = NodeDesiredState {
        schema: NODE_DESIRED_STATE_SCHEMA.to_string(),
        node_id: "node-worker".to_string(),
        role: "builder".to_string(),
        generation: 10,
        policy_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        release: RuntimeSeedRequest::example("node-worker", "cynic-v0.2.0", "75b4cd4b"),
        required_capabilities: vec!["build_runner".to_string()],
        required_services: vec![],
        paused: false,
    };

    let signed_plan = plan_verifier
        .sign_plan(
            desired_state.clone(),
            "kernel-primary",
            &kernel_primary_secret,
            3600,
            now_s,
        )
        .expect("sign plan");

    // Admitted when monotonic (observed generation 9 < plan generation 10)
    let admitted_plan = plan_verifier
        .verify_and_admit_plan(&signed_plan, Some(9), now_s + 10)
        .expect("admit control plan");
    assert_eq!(admitted_plan, desired_state);

    // Stale generation rejected
    let stale_attempt = plan_verifier.verify_and_admit_plan(&signed_plan, Some(10), now_s + 10);
    assert_eq!(
        stale_attempt,
        Err(ControlPlanVerificationStatus::StaleGeneration {
            observed: 10,
            plan: 10
        })
    );

    // =========================================================================
    // Phase 4: Sovereignty, Fenced Mutations & Atomic Standby Takeover
    // =========================================================================
    let mut lease_manager =
        LeaseManager::new(provider.clone(), Some(kernel_primary_public.clone()));

    // Issue initial lease to Node 1 (Cortex) with fencing token 100, TTL 5000ms
    let lease1 = lease_manager
        .issue_lease(
            "lease-001",
            "node-cortex",
            "cortex_primary",
            100,
            5000,
            "kernel-primary",
            &kernel_primary_secret,
            now_ms,
        )
        .expect("issue lease 1");

    lease_manager
        .acquire_or_renew_lease(lease1, now_ms)
        .expect("acquire lease 1");
    assert!(lease_manager.has_active_authority("node-cortex", now_ms + 100));
    assert_eq!(lease_manager.fencing_token_watermark(), 100);

    // Fenced writes permitted during active lease with token >= 100
    assert!(
        lease_manager
            .assert_fenced_write("node-cortex", 100, now_ms + 100)
            .is_ok()
    );
    assert!(
        lease_manager
            .assert_fenced_write("node-cortex", 105, now_ms + 100)
            .is_ok()
    );
    // Stale token 99 rejected
    assert!(
        lease_manager
            .assert_fenced_write("node-cortex", 99, now_ms + 100)
            .is_err()
    );

    // Instant authority drop: At now_ms + 5000, lease expires
    assert!(!lease_manager.has_active_authority("node-cortex", now_ms + 5000));
    assert!(
        lease_manager
            .assert_fenced_write("node-cortex", 100, now_ms + 5000)
            .is_err()
    );

    // Standby Takeover: Node 2 takes over authority with strictly higher fencing token (101)
    let takeover_lease = lease_manager
        .issue_lease(
            "lease-002",
            "node-worker",
            "cortex_primary",
            101, // strictly monotonic
            10000,
            "kernel-primary",
            &kernel_primary_secret,
            now_ms + 5000,
        )
        .expect("issue takeover lease");

    lease_manager
        .acquire_or_renew_lease(takeover_lease, now_ms + 5000)
        .expect("acquire takeover lease");
    assert!(lease_manager.has_active_authority("node-worker", now_ms + 5100));
    assert_eq!(lease_manager.fencing_token_watermark(), 101);

    // Sequenced role transition history is recorded and auditable
    let history = lease_manager.transition_history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].to_node_id, "node-cortex");
    assert_eq!(history[0].fencing_token, 100);
    assert_eq!(history[1].from_node_id, Some("node-cortex".to_string()));
    assert_eq!(history[1].to_node_id, "node-worker");
    assert_eq!(history[1].fencing_token, 101);
}
