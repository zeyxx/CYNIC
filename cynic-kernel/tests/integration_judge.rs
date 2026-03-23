//! Integration test: Stimulus → Judge → Verdict → Storage round-trip
//! Uses DeterministicDog only (no API key needed for CI).

use std::sync::Arc;
use cynic_kernel::domain::dog::*;
use cynic_kernel::domain::health_gate::HealthGate;
use cynic_kernel::dogs::deterministic::DeterministicDog;
use cynic_kernel::domain::metrics::Metrics;
use cynic_kernel::infra::circuit_breaker::CircuitBreaker;
use cynic_kernel::judge::Judge;

fn test_judge(dogs: Vec<Box<dyn Dog>>) -> Judge {
    let breakers: Vec<Arc<dyn HealthGate>> = dogs.iter()
        .map(|d| Arc::new(CircuitBreaker::new(d.id().to_string())) as Arc<dyn HealthGate>)
        .collect();
    Judge::new(dogs, breakers)
}

#[tokio::test]
async fn deterministic_dog_produces_valid_verdict() {
    let judge = test_judge(vec![Box::new(DeterministicDog)]);

    let stimulus = Stimulus {
        content: "According to the data, this approach probably works because of evidence from X".into(),
        context: Some("Testing epistemic humility".into()),
        domain: Some("general".into()),
    };

    let verdict = judge.evaluate(&stimulus, None, &Metrics::new()).await.unwrap();

    // Q-Score must be phi-bounded
    assert!(verdict.q_score.total <= PHI_INV + 1e-10);
    assert!(verdict.q_score.fidelity <= PHI_INV + 1e-10);
    assert!(verdict.q_score.phi <= PHI_INV + 1e-10);
    assert!(verdict.q_score.verify <= PHI_INV + 1e-10);

    // Must have a valid verdict kind
    assert!(matches!(verdict.kind, VerdictKind::Howl | VerdictKind::Wag | VerdictKind::Growl | VerdictKind::Bark));

    // Must have reasoning
    assert!(!verdict.reasoning.fidelity.is_empty());

    // ID must be a valid UUID
    assert_eq!(verdict.id.len(), 36);

    println!("Verdict: {:?} | Q-Score: {:.3} | F:{:.3} Phi:{:.3} V:{:.3}",
        verdict.kind, verdict.q_score.total,
        verdict.q_score.fidelity, verdict.q_score.phi, verdict.q_score.verify);
}

#[tokio::test]
async fn absolute_claim_scores_lower() {
    let judge = test_judge(vec![Box::new(DeterministicDog)]);

    let humble = Stimulus {
        content: "This probably works in most cases according to the data".into(),
        context: None, domain: None,
    };
    let absolute = Stimulus {
        content: "This always works 100% guaranteed never fails".into(),
        context: None, domain: None,
    };

    let v_humble = judge.evaluate(&humble, None, &Metrics::new()).await.unwrap();
    let v_absolute = judge.evaluate(&absolute, None, &Metrics::new()).await.unwrap();

    assert!(v_humble.q_score.total > v_absolute.q_score.total,
        "Humble ({:.3}) should score higher than absolute ({:.3})",
        v_humble.q_score.total, v_absolute.q_score.total);
}
