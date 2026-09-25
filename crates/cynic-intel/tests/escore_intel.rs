use cynic_intel::escore::decay::{
    apply_tier_weight, calculate_decay, calculate_quadratic_modifier, from_points,
};
use cynic_intel::escore::merkle::{build_proof, merkle_root, verify_proof};
use cynic_intel::escore::types::{EScoreSignal, EvidenceTier, SCORE_SCALE, StatType};

fn sample_signal(wallet_id: &str, points: u64, timestamp: i64) -> EScoreSignal {
    EScoreSignal {
        wallet_id: wallet_id.to_owned(),
        stat_type: StatType::Koinonia,
        evidence_tier: EvidenceTier::T1OnChain,
        raw_weight: points * SCORE_SCALE,
        timestamp,
<<<<<<< HEAD
        epoch: None,
=======
>>>>>>> github/main
    }
}

#[test]
fn decay_strictly_decreases_as_inactivity_grows() {
    // 9_200 bps = 92% de rétention par jour
    let checkpoints = [0u32, 3, 7, 14, 30];
    let mut values = checkpoints
        .into_iter()
        .map(|days| calculate_decay(from_points(125), days, 9_200));

    let Some(mut previous) = values.next() else {
        panic!("expected at least one decay checkpoint");
    };
    for current in values {
        assert!(previous > current, "expected {previous} > {current}");
        previous = current;
    }
}

#[test]
fn decay_is_deterministic_across_calls() {
    let a = calculate_decay(from_points(500), 14, 9_500);
    let b = calculate_decay(from_points(500), 14, 9_500);
    assert_eq!(a, b, "Le decay doit être déterministe");
}

#[test]
fn quadratic_modifier_penalizes_relative_scale() {
    let smaller = calculate_quadratic_modifier(from_points(20), from_points(100));
    let larger = calculate_quadratic_modifier(from_points(80), from_points(100));

    assert!(
        smaller < larger,
        "Le petit wallet doit avoir moins de poids"
    );
    assert!(
        larger < 10_000,
        "Même le plus gros wallet ne sature pas le modificateur"
    );
}

#[test]
fn tier_weight_scales_correctly() {
    let raw = from_points(100);
    // T1OnChain = 10_000 bps = 100 % → signal intact
    let t1 = apply_tier_weight(raw, EvidenceTier::T1OnChain.weight_bps());
    // T4Solo    = 2_500 bps  =  25% → signal réduit à 25 points
    let t4 = apply_tier_weight(raw, EvidenceTier::T4Solo.weight_bps());

    assert_eq!(t1, raw);
    assert_eq!(t4, from_points(25));
}

#[test]
fn merkle_root_and_proof_reject_tampering() {
    let signals = vec![
        sample_signal("wallet-a", 1, 1_720_000_100),
        sample_signal("wallet-b", 2, 1_720_000_200),
        sample_signal("wallet-c", 3, 1_720_000_300),
    ];
    let (root, mut proof) = match (merkle_root(&signals), build_proof(&signals, 0)) {
        (Ok(root), Ok(proof)) => (root, proof),
        (Err(error), _) => panic!("failed to build merkle root: {error}"),
        (_, Err(error)) => panic!("failed to build merkle proof: {error}"),
    };

    assert!(verify_proof(&proof, &root));

    proof.leaf_hash[31] ^= 0x0F;
    assert!(!verify_proof(&proof, &root));
}
