use cynic_protocol::anchor_client::{
    ANCHOR_CLIENT_SCHEMA_V1, POH_CHECKPOINT_BATCH_SCHEMA, build_create_futarchy_market_ix,
    build_poh_checkpoint_from_digests, build_record_poh_ix, compute_merkle_root_from_digests,
    derive_pda_digest,
};

#[test]
fn test_anchor_instruction_builders_and_pda_derivation() {
    let program_id = "CYNiC11111111111111111111111111111111111111";
    let node_auth = "NodeAuthPubKey11111111111111111111111111111";

    // 1. Test PoH Checkpoint Instruction
    let poh_ix = build_record_poh_ix(program_id, node_auth, 123, [0xAA; 32], [0xBB; 32]);

    assert_eq!(poh_ix.schema, ANCHOR_CLIENT_SCHEMA_V1);
    assert_eq!(poh_ix.instruction_name, "record_poh_checkpoint");
    assert_eq!(poh_ix.accounts.len(), 3);
    assert!(poh_ix.accounts[0].is_writable);
    assert!(poh_ix.accounts[1].is_signer);

    // 2. Test Futarchy Market Creation Instruction
    let proposal_hash = [0xCC; 32];
    let futarchy_ix =
        build_create_futarchy_market_ix(program_id, node_auth, proposal_hash, 5_000_000, 6000);

    assert_eq!(futarchy_ix.instruction_name, "create_futarchy_market");
    assert_eq!(futarchy_ix.accounts.len(), 3);

    // 3. Test PDA determinism
    let pda1 = derive_pda_digest(&[b"futarchy_market", &proposal_hash], program_id);
    let pda2 = derive_pda_digest(&[b"futarchy_market", &proposal_hash], program_id);
    assert_eq!(pda1, pda2, "PDA derivation must be deterministic");
}

#[test]
fn test_poh_merkle_batching_and_compression() {
    let program_id = "CYNiC11111111111111111111111111111111111111";
    let node_auth = "NodeAuthPubKey11111111111111111111111111111";

    // Simulation of N=4 heterogeneous evidence events
    let evidence_digests = vec![
        "sha256:444e70f19cf0003053d29315c46466bc461d9306c27a78d041416602687df926".to_string(), // BuildProof
        "sha256:244d88ca0dafd0df0732e1afe68b63f8ab8867dad3c6314010cda9db686c5375".to_string(), // Registry
        "sha256:c0f80aaf0a9f8fd92580a1458b41c10b3aec19431df23758afff730dd17a2bd6".to_string(), // NodeIdentity
        "sha256:ae6a6a2f11658de57c245c589f0be6aa5623d19750f556f3b20504932d847662".to_string(), // UniversalEvent
    ];

    let root = compute_merkle_root_from_digests(&evidence_digests);
    assert_ne!(root, [0u8; 32]);

    let batch = build_poh_checkpoint_from_digests(
        program_id,
        node_auth,
        42,
        &evidence_digests,
        1_787_060_000_000,
    );

    assert_eq!(batch.schema, POH_CHECKPOINT_BATCH_SCHEMA);
    assert_eq!(batch.epoch, 42);
    assert_eq!(batch.event_count, 4);
    assert_eq!(batch.instruction.instruction_name, "record_poh_checkpoint");
    assert_eq!(batch.instruction.accounts.len(), 3);
}
