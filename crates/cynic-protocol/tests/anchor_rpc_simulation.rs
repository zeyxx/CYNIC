use cynic_protocol::anchor_client::{
    build_create_futarchy_market_ix, build_record_poh_ix, derive_pda_digest,
    ANCHOR_CLIENT_SCHEMA_V1,
};

#[test]
fn test_anchor_instruction_builders_and_pda_derivation() {
    let program_id = "CYNiC11111111111111111111111111111111111111";
    let node_auth = "NodeAuthPubKey11111111111111111111111111111";

    // 1. Test PoH Checkpoint Instruction
    let poh_ix = build_record_poh_ix(
        program_id,
        node_auth,
        123,
        [0xAA; 32],
        [0xBB; 32],
    );

    assert_eq!(poh_ix.schema, ANCHOR_CLIENT_SCHEMA_V1);
    assert_eq!(poh_ix.instruction_name, "record_poh_checkpoint");
    assert_eq!(poh_ix.accounts.len(), 3);
    assert!(poh_ix.accounts[0].is_writable);
    assert!(poh_ix.accounts[1].is_signer);

    // 2. Test Futarchy Market Creation Instruction
    let proposal_hash = [0xCC; 32];
    let futarchy_ix = build_create_futarchy_market_ix(
        program_id,
        node_auth,
        proposal_hash,
        5_000_000,
        6000,
    );

    assert_eq!(futarchy_ix.instruction_name, "create_futarchy_market");
    assert_eq!(futarchy_ix.accounts.len(), 3);

    // 3. Test PDA determinism
    let pda1 = derive_pda_digest(&[b"futarchy_market", &proposal_hash], program_id);
    let pda2 = derive_pda_digest(&[b"futarchy_market", &proposal_hash], program_id);
    assert_eq!(pda1, pda2, "PDA derivation must be deterministic");
}
