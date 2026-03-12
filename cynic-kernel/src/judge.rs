//! Judge — orchestrates Dogs, computes consensus, emits Verdicts.
//! Currently runs Dogs sequentially (MVP). Future: parallel + BFT consensus.

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

    /// Evaluate a stimulus through all Dogs, aggregate, produce Verdict.
    pub async fn evaluate(&self, stimulus: &Stimulus) -> Result<Verdict, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        let mut all_scores: Vec<(String, AxiomScores)> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // MVP: sequential evaluation. Future: tokio::join! for parallel.
        for dog in &self.dogs {
            match dog.evaluate(stimulus).await {
                Ok(scores) => all_scores.push((dog.id().to_string(), scores)),
                Err(e) => errors.push(format!("{}: {}", dog.id(), e)),
            }
        }

        if all_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(errors));
        }

        // Aggregate: average raw scores across Dogs, then phi-bound
        let n = all_scores.len() as f64;
        let avg_fidelity = all_scores.iter().map(|(_, s)| s.fidelity).sum::<f64>() / n;
        let avg_phi = all_scores.iter().map(|(_, s)| s.phi).sum::<f64>() / n;
        let avg_verify = all_scores.iter().map(|(_, s)| s.verify).sum::<f64>() / n;

        let aggregated = AxiomScores {
            fidelity: avg_fidelity,
            phi: avg_phi,
            verify: avg_verify,
            reasoning: all_scores.last().map(|(_, s)| s.reasoning.clone())
                .unwrap_or_default(),
        };

        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);

        let dog_ids: Vec<&str> = all_scores.iter().map(|(id, _)| id.as_str()).collect();

        Ok(Verdict {
            id: Uuid::new_v4().to_string(),
            kind,
            q_score,
            reasoning: aggregated.reasoning,
            dog_id: dog_ids.join("+"),
            stimulus_summary: stimulus.content.chars().take(100).collect(),
            timestamp: Utc::now().to_rfc3339(),
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
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.q_score.total <= PHI_INV + 1e-10);
        assert!(verdict.q_score.total > 0.0);
        assert_eq!(verdict.dog_id, "test");
    }

    #[tokio::test]
    async fn multiple_dogs_averaged() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8, phi: 0.8, verify: 0.8,
                    reasoning: AxiomReasoning::default(),
                },
            }),
            Box::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2, phi: 0.2, verify: 0.2,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.dog_id.contains("high"));
        assert!(verdict.dog_id.contains("low"));
        // Average should be around 0.5, phi-bounded
        assert!(verdict.q_score.total > 0.3);
        assert!(verdict.q_score.total < 0.55);
    }

    #[tokio::test]
    async fn surviving_dog_still_produces_verdict() {
        let judge = Judge::new(vec![
            Box::new(FailDog),
            Box::new(FixedDog {
                name: "survivor".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
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
}
