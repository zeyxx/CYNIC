use super::types::{
    AxiomScoresBps, EPISTEMIC_VERDICT_SCHEMA_V1, EpistemicVerdict, HOWL_THRESHOLD_BPS,
    MAX_SCORE_BPS, PHI_INV_BPS, PHI_INV_CUBE_BPS, PHI_INV_SQ_BPS, ValidatorDogScore, VerdictRating,
};

pub fn trimmed_mean_bps(mut scores: Vec<u32>) -> u32 {
    if scores.is_empty() {
        return 0;
    }
    if scores.len() < 4 {
        return (scores.iter().map(|&score| u64::from(score)).sum::<u64>() / scores.len() as u64)
            as u32;
    }

    scores.sort_unstable();
    let trimmed = &scores[1..scores.len() - 1];
    (trimmed.iter().map(|&score| u64::from(score)).sum::<u64>() / trimmed.len() as u64) as u32
}

/// Integer geometric mean of six BPS values, rounded down.
///
/// The bounded integer root avoids architecture- and library-dependent floating
/// point behavior in a consensus estimate.
pub fn geometric_mean_6_axioms(axioms: &AxiomScoresBps) -> u32 {
    let product = [
        axioms.fidelity,
        axioms.phi,
        axioms.verify,
        axioms.culture,
        axioms.burn,
        axioms.sovereignty,
    ]
    .into_iter()
    .map(|score| u128::from(score.min(MAX_SCORE_BPS)))
    .product();

    sixth_root_floor(product)
}

pub fn evaluate_epistemic_consensus(
    verdict_id: &str,
    stimulus_id: &str,
    domain: &str,
    dogs: &[ValidatorDogScore],
    timestamp: i64,
) -> Result<EpistemicVerdict, String> {
    if dogs.is_empty() {
        return Err("cannot compute epistemic consensus without validators".to_string());
    }
    if dogs.iter().any(|dog| !dog.scores.all_within_bounds()) {
        return Err("validator score exceeds the BPS range".to_string());
    }

    let consensus_axioms = AxiomScoresBps {
        fidelity: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.fidelity).collect()),
        phi: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.phi).collect()),
        verify: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.verify).collect()),
        culture: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.culture).collect()),
        burn: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.burn).collect()),
        sovereignty: trimmed_mean_bps(dogs.iter().map(|dog| dog.scores.sovereignty).collect()),
    };
    let anomaly_detected = [
        dogs.iter()
            .map(|dog| dog.scores.fidelity)
            .collect::<Vec<_>>(),
        dogs.iter().map(|dog| dog.scores.phi).collect::<Vec<_>>(),
        dogs.iter().map(|dog| dog.scores.verify).collect::<Vec<_>>(),
        dogs.iter()
            .map(|dog| dog.scores.culture)
            .collect::<Vec<_>>(),
        dogs.iter().map(|dog| dog.scores.burn).collect::<Vec<_>>(),
        dogs.iter()
            .map(|dog| dog.scores.sovereignty)
            .collect::<Vec<_>>(),
    ]
    .iter()
    .any(|scores| check_anomaly(scores));
    let q_score_bps = geometric_mean_6_axioms(&consensus_axioms).min(PHI_INV_BPS);
    let rating = rating_for(q_score_bps);

    Ok(EpistemicVerdict {
        schema: EPISTEMIC_VERDICT_SCHEMA_V1.to_string(),
        verdict_id: verdict_id.to_string(),
        stimulus_id: stimulus_id.to_string(),
        domain: domain.to_string(),
        rating,
        q_score_bps,
        consensus_axioms,
        participating_dogs_count: dogs.len(),
        anomaly_detected,
        timestamp,
        residual_unknowns: vec![
            "validator inputs are unverified estimates".to_string(),
            "no external outcome observation is attached".to_string(),
        ],
    })
}

fn sixth_root_floor(product: u128) -> u32 {
    let mut low = 0_u32;
    let mut high = MAX_SCORE_BPS;
    while low < high {
        let midpoint = low + (high - low).div_ceil(2);
        if u128::from(midpoint).pow(6) <= product {
            low = midpoint;
        } else {
            high = midpoint - 1;
        }
    }
    low
}

fn rating_for(q_score_bps: u32) -> VerdictRating {
    if q_score_bps >= HOWL_THRESHOLD_BPS {
        VerdictRating::Howl
    } else if q_score_bps >= PHI_INV_SQ_BPS {
        VerdictRating::Wag
    } else if q_score_bps >= PHI_INV_CUBE_BPS {
        VerdictRating::Growl
    } else {
        VerdictRating::Bark
    }
}

fn check_anomaly(scores: &[u32]) -> bool {
    scores.len() >= 2
        && scores
            .iter()
            .max()
            .zip(scores.iter().min())
            .is_some_and(|(max, min)| max.saturating_sub(*min) > PHI_INV_SQ_BPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn score(value: u32) -> AxiomScoresBps {
        AxiomScoresBps {
            fidelity: value,
            phi: value,
            verify: value,
            culture: value,
            burn: value,
            sovereignty: value,
        }
    }

    #[test]
    fn consensus_is_bounded_and_retains_residual_unknowns() {
        let verdict = evaluate_epistemic_consensus(
            "verdict-1",
            "stimulus-1",
            "simulation",
            &[ValidatorDogScore {
                dog_name: "validator-1".to_string(),
                scores: score(MAX_SCORE_BPS),
                latency_ms: 1,
            }],
            1,
        )
        .expect("valid consensus");
        assert_eq!(verdict.q_score_bps, PHI_INV_BPS);
        assert_eq!(verdict.rating, VerdictRating::Howl);
        assert!(!verdict.residual_unknowns.is_empty());
    }

    #[test]
    fn rejects_out_of_range_input_and_detects_divergence() {
        assert!(
            evaluate_epistemic_consensus(
                "verdict-1",
                "stimulus-1",
                "simulation",
                &[ValidatorDogScore {
                    dog_name: "invalid".to_string(),
                    scores: score(MAX_SCORE_BPS + 1),
                    latency_ms: 1,
                }],
                1,
            )
            .is_err()
        );

        let verdict = evaluate_epistemic_consensus(
            "verdict-2",
            "stimulus-2",
            "simulation",
            &[
                ValidatorDogScore {
                    dog_name: "high".to_string(),
                    scores: score(8_000),
                    latency_ms: 1,
                },
                ValidatorDogScore {
                    dog_name: "low".to_string(),
                    scores: score(2_000),
                    latency_ms: 1,
                },
            ],
            1,
        )
        .expect("valid consensus");
        assert!(verdict.anomaly_detected);
    }
}
