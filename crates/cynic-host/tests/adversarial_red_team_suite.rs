use cynic_host::audit_engine::run_workspace_audit;
use cynic_host::coordination_lease_store::CoordinationLeaseStore;
use cynic_intel::escore::decay::{calculate_decay, calculate_quadratic_modifier};
use cynic_intel::escore::merkle::{build_proof, merkle_root, verify_proof};
use cynic_intel::escore::types::{EScoreSignal, EvidenceTier, SCORE_SCALE, StatType};
use cynic_intel::futarchy::cpmm::{calculate_prices_bps, create_market, execute_bet, resolve_market};
use cynic_intel::futarchy::types::{MarketDecision, PredictionBet, ProposalStatus};
use cynic_protocol::{LeaseAction, LeaseGuard, TargetClaimOutcome};
use std::path::Path;
use std::sync::Arc;
use std::thread;

/// ── VECTEUR 1 : Course Critique & Attaque par Zombie Write ─────────────────
#[test]
fn test_adversarial_concurrent_lease_claims_and_zombie_writes() {
    let store = Arc::new(CoordinationLeaseStore::new());
    let target = "crates/cynic-kernel/state_bus";
    let num_threads = 32;
    let mut handles = Vec::new();

    // 32 threads tentent simultanément de revendiquer le MÊME domaine
    for i in 0..num_threads {
        let store_clone = Arc::clone(&store);
        let agent_id = format!("adversary-agent-{i}");
        handles.push(thread::spawn(move || {
            store_clone.claim(&agent_id, target, 3600)
        }));
    }

    let mut claimed_count = 0;
    let mut conflict_count = 0;
    let mut winner_grant = None;
    let mut winner_agent = None;

    for (i, h) in handles.into_iter().enumerate() {
        let outcome = h.join().expect("thread join");
        match outcome {
            TargetClaimOutcome::Claimed { grant, .. } => {
                claimed_count += 1;
                winner_grant = Some(grant);
                winner_agent = Some(format!("adversary-agent-{i}"));
            }
            TargetClaimOutcome::Conflict { .. } => {
                conflict_count += 1;
            }
        }
    }

    // Invariant fondamental : EXACTEMENT 1 gagnant, 31 conflits
    assert_eq!(claimed_count, 1, "There must be exactly ONE winner");
    assert_eq!(conflict_count, num_threads - 1, "All other concurrent claims must conflict");

    let grant = winner_grant.expect("winner grant");
    let agent_id = winner_agent.expect("winner agent");

    // Attaque Zombie Write : Un attaquant tente de muter avec un token forgé ou périmé
    let forged_guard = LeaseGuard {
        agent_id: "impostor-agent".to_string(),
        target: target.to_string(),
        lease_id: grant.lease_id.clone(),
        fencing_token: grant.fencing_token,
    };
    let reject_impostor = store.mutate(&forged_guard, LeaseAction::Release, None, None);
    assert!(!reject_impostor.is_applied(), "Impostor mutation must be rejected");

    let stale_guard = LeaseGuard {
        agent_id: agent_id.clone(),
        target: target.to_string(),
        lease_id: grant.lease_id.clone(),
        fencing_token: grant.fencing_token.saturating_sub(1), // Stale token
    };
    let reject_stale = store.mutate(&stale_guard, LeaseAction::Release, None, None);
    assert!(!reject_stale.is_applied(), "Stale fencing token must be rejected");

    // Mutation légitime par le vrai détenteur
    let legitimate_guard = LeaseGuard {
        agent_id,
        target: target.to_string(),
        lease_id: grant.lease_id,
        fencing_token: grant.fencing_token,
    };
    let accept_legit = store.mutate(&legitimate_guard, LeaseAction::Release, None, None);
    assert!(accept_legit.is_applied(), "Legitimate release must succeed");
}

/// ── VECTEUR 2 : Fuzzing CPMM & Protection contre l'Exploit de Liquidité ─────
#[test]
fn test_adversarial_cpmm_extreme_bounds_and_overflow_resistance() {
    let mut market = create_market(
        "mkt-adversarial",
        "prop-adversarial",
        "Can CPMM handle extreme volume without overflow?",
        100_000_000_000, // 100 milliards d'unités
        7_000,           // 70% threshold
    );

    // 1. Pari à 0 unité -> Doit être rejeté
    let zero_bet = PredictionBet {
        bettor_wallet: "attacker-zero".to_string(),
        is_yes: true,
        amount_units: 0,
        timestamp: 1_720_000_000,
    };
    assert!(execute_bet(&mut market, zero_bet).is_err(), "Zero bet must fail");

    // 2. Pari colossal (100 milliards) -> Calcul u128 sans overflow
    let whale_bet = PredictionBet {
        bettor_wallet: "whale-attacker".to_string(),
        is_yes: true,
        amount_units: 100_000_000_000,
        timestamp: 1_720_000_001,
    };
    let shares = execute_bet(&mut market, whale_bet).expect("Whale bet execution must not panic");
    assert!(shares > 0);

    // Les prix doivent rester bornés entre 0 et 10 000 bps
    let (p_yes, p_no) = calculate_prices_bps(market.yes_pool_units, market.no_pool_units);
    assert_eq!(p_yes + p_no, 10_000, "Price sum must strictly equal 10,000 bps");
    assert!(p_yes > 7_000, "Price of YES should reflect massive buying pressure");

    // Résolution
    let resolution = resolve_market(&mut market, 1_720_000_100);
    assert_eq!(resolution.decision, MarketDecision::Pass);
    assert_eq!(market.status, ProposalStatus::Resolved);

    // 3. Tentative de pari post-clôture -> Rejet
    let post_close_bet = PredictionBet {
        bettor_wallet: "late-attacker".to_string(),
        is_yes: false,
        amount_units: 1_000_000,
        timestamp: 1_720_000_105,
    };
    assert!(
        execute_bet(&mut market, post_close_bet).is_err(),
        "Bet on resolved market must fail"
    );
}

/// ── VECTEUR 3 : Falsification Merkle & Attaque Sybil sur l'E-Score ──────────
#[test]
fn test_adversarial_merkle_tampering_and_sybil_resistance() {
    let signals = vec![
        EScoreSignal {
            wallet_id: "wallet-sybil-1".to_string(),
            stat_type: StatType::Karteria,
            evidence_tier: EvidenceTier::T1OnChain,
            raw_weight: 10 * SCORE_SCALE,
            timestamp: 1_720_000_000,
        },
        EScoreSignal {
            wallet_id: "wallet-sybil-2".to_string(),
            stat_type: StatType::Parrhesia,
            evidence_tier: EvidenceTier::T1OnChain,
            raw_weight: 10 * SCORE_SCALE,
            timestamp: 1_720_000_000,
        },
    ];

    let root = merkle_root(&signals).expect("merkle root");
    let proof = build_proof(&signals, 0).expect("merkle proof");

    // 1. Vérification légitime
    assert!(verify_proof(&proof, &root));

    // 2. Falsification du hash de feuille
    let mut fake_leaf_proof = proof.clone();
    fake_leaf_proof.leaf_hash[0] ^= 0xFF;
    assert!(!verify_proof(&fake_leaf_proof, &root), "Tampered leaf hash must be rejected");

    // 3. Falsification du chemin Merkle
    let mut fake_step_proof = proof;
    fake_step_proof.path[0].sibling_hash[0] ^= 0xFF;
    assert!(!verify_proof(&fake_step_proof, &root), "Tampered path step must be rejected");

    // 4. Propriété anti-Sybil du modificateur quadratique
    // Deux petits comptes de 10 points chacun doivent rapporter moins d'influence qu'un compte consolidé de 20 points
    let mod_single_10 = calculate_quadratic_modifier(10 * SCORE_SCALE, 100 * SCORE_SCALE);
    let mod_single_20 = calculate_quadratic_modifier(20 * SCORE_SCALE, 100 * SCORE_SCALE);
    assert!(
        mod_single_20 > mod_single_10,
        "Higher conviction wallet must have strictly higher weight than single small wallet"
    );

    // 5. Décroissance temporelle face à des timestamps distants
    let decayed_100_days = calculate_decay(100 * SCORE_SCALE, 100, 9_200);
    assert!(
        decayed_100_days < 10 * SCORE_SCALE,
        "100 days of inactivity at 92% retention must decay > 90% of the score"
    );
}

/// ── VECTEUR 4 : Audit AST v2 face à des Évasions et Faux Positifs ───────────
#[test]
fn test_adversarial_workspace_audit_resilience() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace crates")
        .parent()
        .expect("workspace root");

    let report = run_workspace_audit(root);

    // L'audit doit garantir ZÉRO violation critique (fuite de machine path ou import d'adapter interdit dans protocol)
    assert_eq!(
        report.summary.critical_count, 0,
        "Red Team: workspace must have 0 critical security/boundary violations"
    );
}
