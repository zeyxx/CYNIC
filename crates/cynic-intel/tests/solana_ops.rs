use cynic_intel::solana::{
    AllowanceGrantPaymentExt, SolanaPayRequest, SolanaTakeoverAttestation,
    AMM_PROGRAMS, BURN_ADDRESSES, INFRA_PROGRAMS, LOCKER_PROGRAMS, ORACLE_PROGRAMS,
};
use cynic_protocol::allowance::AllowanceGrant;

#[test]
fn solana_pay_url_generation() {
    let req = SolanaPayRequest::new("7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c")
        .with_amount(12.5)
        .with_spl_token("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")
        .with_reference("RefKey123")
        .with_label("Store Purchase")
        .with_message("Order #412");

    let url = req.to_url_string().unwrap();
    assert!(url.starts_with("solana:7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c"));
    assert!(url.contains("amount=12.5"));
    assert!(url.contains("spl-token=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
    assert!(url.contains("reference=RefKey123"));
    assert!(url.contains("label=Store+Purchase"));
}

#[test]
fn allowance_grant_spend_limit_check() {
    let grant = AllowanceGrant {
        grant_id: "grant-1".into(),
        owner_pubkey: "owner1".into(),
        delegate_pubkey: "agent1".into(),
        spl_token: Some("USDC".into()),
        max_amount_per_period: 100.0,
        period_seconds: 86400,
        remaining_allowance: 50.0,
        allowlist_recipients: vec!["vendor1".into()],
        allowlist_mints: vec!["USDC".into()],
        expires_at: 1900000000,
    };

    assert!(grant.validate_transfer("vendor1", 20.0, Some("USDC")).is_ok());
    assert!(grant.validate_transfer("vendor1", 60.0, Some("USDC")).is_err());
    assert!(grant.validate_transfer("attacker", 10.0, Some("USDC")).is_err());
}

#[test]
fn solana_takeover_attestation_derivation() {
    let record = cynic_protocol::runtime_takeover::RoleTransitionRecord::new(
        "tx-001".to_string(),
        Some("node-a".to_string()),
        "node-b".to_string(),
        "cortex_primary".to_string(),
        42,
        41,
        10000,
        "a".repeat(128),
    );

    let attestation =
        SolanaTakeoverAttestation::from_transition_record(&record, "mainnet-beta");
    assert_eq!(attestation.fencing_token, 42);
    assert_eq!(attestation.to_node_id, "node-b");
    assert!(attestation.commitment_memo.contains("token=42:prior=41"));
    assert!(attestation.validate().is_empty());
}

#[test]
fn program_constants_coherence() {
    assert!(!AMM_PROGRAMS.is_empty());
    assert!(!BURN_ADDRESSES.is_empty());
    assert!(!INFRA_PROGRAMS.is_empty());
    assert!(!LOCKER_PROGRAMS.is_empty());
    assert!(!ORACLE_PROGRAMS.is_empty());
}
