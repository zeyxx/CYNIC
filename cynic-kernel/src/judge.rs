//! Judge — orchestrates Dogs, computes consensus, emits Verdicts.
//! Parallel evaluation via futures::future::join_all.
//! Residual detection: disagreement > φ⁻² = ANOMALY.

use crate::dog::*;
use chrono::Utc;
use uuid::Uuid;

pub struct Judge {
    dogs: Vec<Box<dyn Dog>>,
}

impl Judge {
    pub fn new(dogs: Vec<Box<dyn Dog>>) -> Self {
        Self { dogs }
    }

    /// Evaluate a stimulus through all Dogs in parallel, aggregate, produce Verdict.
    pub async fn evaluate(&self, stimulus: &Stimulus) -> Result<Verdict, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Parallel evaluation — all Dogs run concurrently
        let futures: Vec<_> = self.dogs.iter()
            .map(|dog| {
                let id = dog.id().to_string();
                async move {
                    let result = dog.evaluate(stimulus).await;
                    (id, result)
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut dog_scores: Vec<DogScore> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for (id, result) in results {
            match result {
                Ok(scores) => {
                    dog_scores.push(DogScore {
                        dog_id: id,
                        fidelity: scores.fidelity,
                        phi: scores.phi,
                        verify: scores.verify,
                        culture: scores.culture,
                        burn: scores.burn,
                        sovereignty: scores.sovereignty,
                        reasoning: scores.reasoning,
                    });
                }
                Err(e) => {
                    eprintln!("[Judge] Dog '{}' failed: {}", id, e);
                    errors.push(format!("{}: {}", id, e));
                }
            }
        }

        if dog_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(errors));
        }

        // Aggregate: arithmetic mean of per-axiom raw scores, then compute_qscore applies
        // geometric mean internally (Q = ³√(F×Φ×V)). This matches the spec: "geometric mean
        // (phi-bounded)" refers to compute_qscore, not to how we aggregate across Dogs.
        let n = dog_scores.len() as f64;
        let avg_fidelity = dog_scores.iter().map(|s| s.fidelity).sum::<f64>() / n;
        let avg_phi = dog_scores.iter().map(|s| s.phi).sum::<f64>() / n;
        let avg_verify = dog_scores.iter().map(|s| s.verify).sum::<f64>() / n;
        let avg_culture = dog_scores.iter().map(|s| s.culture).sum::<f64>() / n;
        let avg_burn = dog_scores.iter().map(|s| s.burn).sum::<f64>() / n;
        let avg_sovereignty = dog_scores.iter().map(|s| s.sovereignty).sum::<f64>() / n;

        // Use median Dog's reasoning (deterministic under parallel execution)
        let mut sorted_by_q: Vec<&DogScore> = dog_scores.iter().collect();
        sorted_by_q.sort_by(|a, b| {
            let qa = compute_qscore(&AxiomScores { fidelity: a.fidelity, phi: a.phi, verify: a.verify, culture: a.culture, burn: a.burn, sovereignty: a.sovereignty, reasoning: AxiomReasoning::default() }).total;
            let qb = compute_qscore(&AxiomScores { fidelity: b.fidelity, phi: b.phi, verify: b.verify, culture: b.culture, burn: b.burn, sovereignty: b.sovereignty, reasoning: AxiomReasoning::default() }).total;
            qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let median_reasoning = sorted_by_q.get(sorted_by_q.len() / 2)
            .map(|s| s.reasoning.clone())
            .unwrap_or_default();

        let aggregated = AxiomScores {
            fidelity: avg_fidelity,
            phi: avg_phi,
            verify: avg_verify,
            culture: avg_culture,
            burn: avg_burn,
            sovereignty: avg_sovereignty,
            reasoning: median_reasoning,
        };

        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);

        // Residual detection: compare per-Dog Q-Scores in verdict space
        let consensus_q = q_score.total;
        let max_disagreement = if dog_scores.len() > 1 {
            dog_scores.iter()
                .map(|s| {
                    let dog_raw = AxiomScores {
                        fidelity: s.fidelity, phi: s.phi, verify: s.verify,
                        culture: s.culture, burn: s.burn, sovereignty: s.sovereignty,
                        reasoning: AxiomReasoning::default(),
                    };
                    let dog_q = compute_qscore(&dog_raw).total;
                    (dog_q - consensus_q).abs()
                })
                .fold(0.0_f64, f64::max)
        } else {
            0.0
        };
        let anomaly_detected = max_disagreement > PHI_INV2;

        // Find which axiom has the largest inter-Dog spread (for anomaly_axiom)
        let anomaly_axiom = if anomaly_detected && dog_scores.len() > 1 {
            let axioms = ["fidelity", "phi", "verify", "culture", "burn", "sovereignty"];
            let spreads: Vec<(f64, &str)> = axioms.iter().map(|&name| {
                let values: Vec<f64> = dog_scores.iter().map(|s| match name {
                    "fidelity" => s.fidelity,
                    "phi" => s.phi,
                    "verify" => s.verify,
                    "culture" => s.culture,
                    "burn" => s.burn,
                    "sovereignty" => s.sovereignty,
                    _ => 0.0,
                }).collect();
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                (max - min, name)
            }).collect();
            spreads.into_iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, name)| name.to_string())
        } else {
            None
        };

        let dog_ids: Vec<&str> = dog_scores.iter().map(|s| s.dog_id.as_str()).collect();

        Ok(Verdict {
            id: Uuid::new_v4().to_string(),
            kind,
            q_score,
            reasoning: aggregated.reasoning,
            dog_id: dog_ids.join("+"),
            stimulus_summary: stimulus.content.chars().take(100).collect(),
            timestamp: Utc::now().to_rfc3339(),
            dog_scores,
            anomaly_detected,
            max_disagreement,
            anomaly_axiom,
        })
    }
}

#[derive(Debug)]
pub enum JudgeError {
    NoDogs,
    AllDogsFailed(Vec<String>),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDogs => write!(f, "No Dogs configured"),
            Self::AllDogsFailed(errs) => write!(f, "All Dogs failed: {}", errs.join("; ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A test Dog that returns fixed scores
    struct FixedDog {
        name: String,
        scores: AxiomScores,
    }

    #[async_trait::async_trait]
    impl Dog for FixedDog {
        fn id(&self) -> &str { &self.name }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Ok(self.scores.clone())
        }
    }

    struct FailDog;

    #[async_trait::async_trait]
    impl Dog for FailDog {
        fn id(&self) -> &str { "fail-dog" }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Err(DogError::ApiError("test failure".into()))
        }
    }

    fn test_stimulus() -> Stimulus {
        Stimulus {
            content: "e4 e5 Nf3".into(),
            context: None,
            domain: Some("chess".into()),
        }
    }

    #[tokio::test]
    async fn single_dog_produces_verdict() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "test".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
                    culture: 0.5, burn: 0.5, sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.q_score.total <= PHI_INV + 1e-10);
        assert!(verdict.q_score.total > 0.0);
        assert_eq!(verdict.dog_id, "test");
        assert_eq!(verdict.dog_scores.len(), 1);
        assert!(!verdict.anomaly_detected);
    }

    #[tokio::test]
    async fn multiple_dogs_averaged() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8, phi: 0.8, verify: 0.8,
                    culture: 0.8, burn: 0.8, sovereignty: 0.8,
                    reasoning: AxiomReasoning::default(),
                },
            }),
            Box::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2, phi: 0.2, verify: 0.2,
                    culture: 0.2, burn: 0.2, sovereignty: 0.2,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.dog_id.contains("high"));
        assert!(verdict.dog_id.contains("low"));
        assert!(verdict.q_score.total > 0.3);
        assert!(verdict.q_score.total < 0.55);
        assert_eq!(verdict.dog_scores.len(), 2);
    }

    #[tokio::test]
    async fn surviving_dog_still_produces_verdict() {
        let judge = Judge::new(vec![
            Box::new(FailDog),
            Box::new(FixedDog {
                name: "survivor".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
                    culture: 0.5, burn: 0.5, sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert_eq!(verdict.dog_id, "survivor");
    }

    #[tokio::test]
    async fn all_dogs_fail_returns_error() {
        let judge = Judge::new(vec![Box::new(FailDog)]);
        let result = judge.evaluate(&test_stimulus()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn no_dogs_returns_error() {
        let judge = Judge::new(vec![]);
        let result = judge.evaluate(&test_stimulus()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn residual_detection_flags_high_disagreement() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "optimist".into(),
                scores: AxiomScores {
                    fidelity: 0.9, phi: 0.9, verify: 0.9,
                    culture: 0.9, burn: 0.9, sovereignty: 0.9,
                    reasoning: AxiomReasoning::default(),
                },
            }),
            Box::new(FixedDog {
                name: "pessimist".into(),
                scores: AxiomScores {
                    fidelity: 0.1, phi: 0.1, verify: 0.1,
                    culture: 0.1, burn: 0.1, sovereignty: 0.1,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.anomaly_detected, "Dogs disagreeing 0.9 vs 0.1 should trigger anomaly");
        assert!(verdict.max_disagreement > PHI_INV2);
        assert_eq!(verdict.dog_scores.len(), 2);
        assert!(verdict.anomaly_axiom.is_some());
    }

    #[tokio::test]
    async fn no_anomaly_when_dogs_agree() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "a".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
                    culture: 0.5, burn: 0.5, sovereignty: 0.5,
                    reasoning: AxiomReasoning::default(),
                },
            }),
            Box::new(FixedDog {
                name: "b".into(),
                scores: AxiomScores {
                    fidelity: 0.52, phi: 0.48, verify: 0.51,
                    culture: 0.49, burn: 0.51, sovereignty: 0.50,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(!verdict.anomaly_detected, "Similar scores should not trigger anomaly");
        assert!(verdict.anomaly_axiom.is_none());
    }
}
