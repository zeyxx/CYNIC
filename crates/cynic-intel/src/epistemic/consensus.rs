use super::types::{
    AxiomScoresBps, EPISTEMIC_VERDICT_SCHEMA_V1, EpistemicVerdict, HOWL_THRESHOLD_BPS, PHI_INV_BPS,
    PHI_INV_CUBE_BPS, PHI_INV_SQ_BPS, ValidatorDogScore, VerdictRating,
};

/// Calcul de la moyenne tronquée (Trimmed Mean) pour une série de scores en BPS.
/// Élimine la note la plus haute et la plus basse si >= 4 votants.
pub fn trimmed_mean_bps(mut scores: Vec<u32>) -> u32 {
    if scores.is_empty() {
        return 0;
    }
    if scores.len() == 1 {
        return scores[0];
    }
    if scores.len() < 4 {
        let sum: u64 = scores.iter().map(|&s| s as u64).sum();
        return (sum / scores.len() as u64) as u32;
    }

    scores.sort_unstable();
    // Élimine le min et le max
    let trimmed = &scores[1..scores.len() - 1];
    let sum: u64 = trimmed.iter().map(|&s| s as u64).sum();
    (sum / trimmed.len() as u64) as u32
}

/// Calcul de la moyenne géométrique de 6 axiomes en arithmétique 100% entière.
/// (f1 * f2 * f3 * f4 * f5 * f6)^(1/6)
pub fn geometric_mean_6_axioms(axioms: &AxiomScoresBps) -> u32 {
    // Normaliser sur une échelle log pour éviter le débordement d'un produit à 6 termes
    let a1 = axioms.fidelity.max(1) as f64;
    let a2 = axioms.phi.max(1) as f64;
    let a3 = axioms.verify.max(1) as f64;
    let a4 = axioms.culture.max(1) as f64;
    let a5 = axioms.burn.max(1) as f64;
    let a6 = axioms.sovereignty.max(1) as f64;

    let log_sum = a1.ln() + a2.ln() + a3.ln() + a4.ln() + a5.ln() + a6.ln();
    let geo_mean = (log_sum / 6.0).exp();

    (geo_mean.round() as u32).min(10_000)
}

pub fn evaluate_epistemic_consensus(
    verdict_id: &str,
    stimulus_id: &str,
    domain: &str,
    dogs: &[ValidatorDogScore],
    timestamp: i64,
) -> Result<EpistemicVerdict, String> {
    if dogs.is_empty() {
        return Err("Cannot compute epistemic consensus without validator dogs".to_string());
    }

    let mut fid_scores = Vec::with_capacity(dogs.len());
    let mut phi_scores = Vec::with_capacity(dogs.len());
    let mut ver_scores = Vec::with_capacity(dogs.len());
    let mut cul_scores = Vec::with_capacity(dogs.len());
    let mut bur_scores = Vec::with_capacity(dogs.len());
    let mut sov_scores = Vec::with_capacity(dogs.len());

    for dog in dogs {
        fid_scores.push(dog.scores.fidelity);
        phi_scores.push(dog.scores.phi);
        ver_scores.push(dog.scores.verify);
        cul_scores.push(dog.scores.culture);
        bur_scores.push(dog.scores.burn);
        sov_scores.push(dog.scores.sovereignty);
    }

    // 1. Consensus par moyenne tronquée sur chaque axiome
    let consensus_axioms = AxiomScoresBps {
        fidelity: trimmed_mean_bps(fid_scores.clone()),
        phi: trimmed_mean_bps(phi_scores.clone()),
        verify: trimmed_mean_bps(ver_scores.clone()),
        culture: trimmed_mean_bps(cul_scores.clone()),
        burn: trimmed_mean_bps(bur_scores.clone()),
        sovereignty: trimmed_mean_bps(sov_scores.clone()),
    };

    // 2. Détection d'anomalie résiduelle si divergence > φ⁻² (3820 bps)
    let anomaly = check_anomaly(&fid_scores)
        || check_anomaly(&phi_scores)
        || check_anomaly(&ver_scores)
        || check_anomaly(&cul_scores)
        || check_anomaly(&bur_scores)
        || check_anomaly(&sov_scores);

    // 3. Moyenne géométrique globale
    let raw_geo_mean = geometric_mean_6_axioms(&consensus_axioms);

    // 4. Bornage d'Or structurel : QScore <= φ⁻¹ (61.8034%)
    let q_score_bps = raw_geo_mean.min(PHI_INV_BPS);

    // 5. Classification du Verdict
    let rating = if q_score_bps >= HOWL_THRESHOLD_BPS {
        VerdictRating::Howl
    } else if q_score_bps >= PHI_INV_SQ_BPS {
        VerdictRating::Wag
    } else if q_score_bps >= PHI_INV_CUBE_BPS {
        VerdictRating::Growl
    } else {
        VerdictRating::Bark
    };

    Ok(EpistemicVerdict {
        schema: EPISTEMIC_VERDICT_SCHEMA_V1.to_string(),
        verdict_id: verdict_id.to_string(),
        stimulus_id: stimulus_id.to_string(),
        domain: domain.to_string(),
        rating,
        q_score_bps,
        consensus_axioms,
        participating_dogs_count: dogs.len(),
        anomaly_detected: anomaly,
        timestamp,
    })
}

fn check_anomaly(scores: &[u32]) -> bool {
    if scores.len() < 2 {
        return false;
    }
    let min = *scores.iter().min().unwrap_or(&0);
    let max = *scores.iter().max().unwrap_or(&0);
    (max.saturating_sub(min)) > PHI_INV_SQ_BPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q_score_is_strictly_phi_bounded() {
        let perfect_dog = ValidatorDogScore {
            dog_name: "perfect-dog".to_string(),
            scores: AxiomScoresBps {
                fidelity: 10_000,
                phi: 10_000,
                verify: 10_000,
                culture: 10_000,
                burn: 10_000,
                sovereignty: 10_000,
            },
            latency_ms: 10,
        };

        let verdict = evaluate_epistemic_consensus(
            "vrd-001",
            "stimulus-test",
            "chess",
            &[perfect_dog],
            1_720_000_000,
        )
        .expect("consensus");

        // Même avec des scores parfaits (100%), le Q-Score est borné à 6180 bps (61.8%)
        assert_eq!(verdict.q_score_bps, PHI_INV_BPS);
        assert_eq!(verdict.rating, VerdictRating::Howl);
    }

    #[test]
    fn test_anomaly_detection_triggers_on_high_divergence() {
        let dog_a = ValidatorDogScore {
            dog_name: "dog-a".to_string(),
            scores: AxiomScoresBps {
                fidelity: 8000,
                phi: 5000,
                verify: 5000,
                culture: 5000,
                burn: 5000,
                sovereignty: 5000,
            },
            latency_ms: 10,
        };
        let dog_b = ValidatorDogScore {
            dog_name: "dog-b".to_string(),
            scores: AxiomScoresBps {
                fidelity: 2000, // Divergence = 8000 - 2000 = 6000 > 3820
                phi: 5000,
                verify: 5000,
                culture: 5000,
                burn: 5000,
                sovereignty: 5000,
            },
            latency_ms: 20,
        };

        let verdict = evaluate_epistemic_consensus(
            "vrd-002",
            "stimulus-anomaly",
            "trading",
            &[dog_a, dog_b],
            1_720_000_000,
        )
        .expect("consensus");

        assert!(
            verdict.anomaly_detected,
            "Anomaly must be flagged when divergence > phi^-2"
        );
    }
}
