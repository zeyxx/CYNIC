use cynic_host::audit_engine::run_workspace_audit;
use cynic_host::coordination_lease_store::CoordinationLeaseStore;
use cynic_intel::futarchy::cpmm::{calculate_prices_bps, create_market, execute_bet, resolve_market};
use cynic_intel::futarchy::types::{MarketDecision, PredictionBet, ProposalStatus};
use cynic_intel::futarchy::vault::LiquidityVault;
use cynic_protocol::{LeaseAction, LeaseGuard, TargetClaimOutcome};
use std::path::Path;

#[test]
fn test_full_multi_agent_coordination_and_futarchy_workflow() {
    // 1. Initialiser le store de baux
    let lease_store = CoordinationLeaseStore::new();

    // Agent 1 (Codex) réserve cynic-inference
    let claim_codex = lease_store.claim("agent-codex", "crates/cynic-inference", 3600);
    let grant_codex = match claim_codex {
        TargetClaimOutcome::Claimed { grant, .. } => grant,
        _ => panic!("Expected agent-codex to claim lease successfully"),
    };

    // Agent 2 (Claude) tente de réserver le même domaine -> Conflit attendu
    let claim_claude_conflict = lease_store.claim("agent-claude", "crates/cynic-inference", 3600);
    assert!(
        matches!(claim_claude_conflict, TargetClaimOutcome::Conflict { .. }),
        "Expected conflict on already claimed target"
    );

    // Agent 2 bascule sur cynic-intel -> Succès
    let claim_claude = lease_store.claim("agent-claude", "crates/cynic-intel", 3600);
    let grant_claude = match claim_claude {
        TargetClaimOutcome::Claimed { grant, .. } => grant,
        _ => panic!("Expected agent-claude to claim cynic-intel"),
    };

    // Agent 3 (Pi) réserve docs/specs -> Succès
    let claim_pi = lease_store.claim("agent-pi", "docs/specs", 3600);
    let grant_pi = match claim_pi {
        TargetClaimOutcome::Claimed { grant, .. } => grant,
        _ => panic!("Expected agent-pi to claim docs/specs"),
    };

    // Vérifier que 3 baux actifs sont recensés
    let active_leases = lease_store.who();
    assert_eq!(active_leases.len(), 3);

    // 2. Audit AST v2 exécuté par les agents sur le workspace
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates")
        .parent()
        .expect("workspace root");

    let audit_report = run_workspace_audit(root);
    assert_eq!(
        audit_report.summary.critical_count, 0,
        "Audit v2 must report 0 critical violations during agent collaboration"
    );

    // 3. Marché de Futarchie & Coffre Odysseum
    let mut vault = LiquidityVault::new("vault-odysseum-genesis", 50_000_000);
    let mut market = create_market(
        "mkt-anchor-upgrade",
        "prop-upgrade-anchor-0-30",
        "Should CYNIC adopt Solana Anchor 0.30+ for on-chain anchoring?",
        2_000_000,
        6_000, // 60% threshold
    );

    vault.allocate_market_liquidity(market.clone()).expect("seed market from vault");

    // Agent 1 parie OUI (500 000 unités)
    let bet1 = PredictionBet {
        bettor_wallet: "wallet-codex".to_string(),
        is_yes: true,
        amount_units: 500_000,
        timestamp: 1_720_000_000,
    };
    let shares1 = execute_bet(&mut market, bet1).expect("execute bet 1");
    assert!(shares1 > 0);

    // Agent 2 parie OUI (300 000 unités)
    let bet2 = PredictionBet {
        bettor_wallet: "wallet-claude".to_string(),
        is_yes: true,
        amount_units: 300_000,
        timestamp: 1_720_000_050,
    };
    let shares2 = execute_bet(&mut market, bet2).expect("execute bet 2");
    assert!(shares2 > 0);

    // Résolution du marché
    let resolution = resolve_market(&mut market, 1_720_000_100);
    assert_eq!(market.status, ProposalStatus::Resolved);
    assert_eq!(resolution.decision, MarketDecision::Pass);
    assert!(resolution.yes_price_bps >= 6000);

    // 4. Finalisation des baux avec preuve d'action
    let guard_codex = LeaseGuard {
        agent_id: "agent-codex".to_string(),
        target: "crates/cynic-inference".to_string(),
        lease_id: grant_codex.lease_id,
        fencing_token: grant_codex.fencing_token,
    };
    let final_codex = lease_store.mutate(
        &guard_codex,
        LeaseAction::Finalize,
        None,
        Some("proof-sha256-inference-ok".to_string()),
    );
    assert!(final_codex.is_applied());

    let guard_claude = LeaseGuard {
        agent_id: "agent-claude".to_string(),
        target: "crates/cynic-intel".to_string(),
        lease_id: grant_claude.lease_id,
        fencing_token: grant_claude.fencing_token,
    };
    let final_claude = lease_store.mutate(
        &guard_claude,
        LeaseAction::Finalize,
        None,
        Some("proof-sha256-intel-ok".to_string()),
    );
    assert!(final_claude.is_applied());

    let guard_pi = LeaseGuard {
        agent_id: "agent-pi".to_string(),
        target: "docs/specs".to_string(),
        lease_id: grant_pi.lease_id,
        fencing_token: grant_pi.fencing_token,
    };
    let final_pi = lease_store.mutate(
        &guard_pi,
        LeaseAction::Release,
        None,
        None,
    );
    assert!(final_pi.is_applied());

    // Vérifier que tous les baux sont libérés
    assert_eq!(lease_store.who().len(), 0);
}
