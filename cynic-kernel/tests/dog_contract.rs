//! Dog trait contract tests — any Dog implementation must pass these.

use cynic_kernel::dogs::deterministic::DeterministicDog;
use cynic_kernel::dogs::inference::InferenceDog;
use cynic_kernel::domain::chat::MockChatBackend;
use cynic_kernel::domain::dog::*;
use std::sync::Arc;

async fn dog_contract(dog: &dyn Dog) {
    // 1. ID must be non-empty
    assert!(!dog.id().is_empty(), "Dog must have non-empty id");

    // 2. Evaluate a simple stimulus
    let stimulus = Stimulus {
        content: "The Earth orbits the Sun, based on centuries of astronomical observation.".into(),
        context: None,
        domain: Some("science".into()),
        request_id: None,
    };

    let result = dog.evaluate(&stimulus).await;
    match result {
        Ok(scores) => {
            // Scores must be in [0.0, 1.0]
            assert!(
                scores.fidelity >= 0.0 && scores.fidelity <= 1.0,
                "fidelity {} out of range",
                scores.fidelity
            );
            assert!(
                scores.phi >= 0.0 && scores.phi <= 1.0,
                "phi {} out of range",
                scores.phi
            );
            assert!(
                scores.verify >= 0.0 && scores.verify <= 1.0,
                "verify {} out of range",
                scores.verify
            );
        }
        Err(DogError::ApiError(_)) => {}             // acceptable
        Err(DogError::ParseError(_)) => {}           // acceptable
        Err(DogError::ZeroFlood(_)) => {}            // acceptable
        Err(DogError::DegenerateScores { .. }) => {} // acceptable
        Err(DogError::RateLimited(_)) => {}          // acceptable
        Err(DogError::Timeout) => {}                 // acceptable
        Err(DogError::ContextOverflow { .. }) => {}  // acceptable
    }
}

#[tokio::test]
async fn deterministic_dog_passes_contract() {
    dog_contract(&DeterministicDog).await;
}

#[tokio::test]
async fn inference_dog_with_mock_passes_contract() {
    let mock = Arc::new(MockChatBackend::new(
        "mock",
        r#"{"fidelity": 0.7, "phi": 0.6, "verify": 0.5, "fidelity_reason": "r1", "phi_reason": "r2", "verify_reason": "r3"}"#,
    ));
    let dog = InferenceDog::new(
        mock,
        "mock-dog".into(),
        4096,
        30,
        cynic_kernel::infra::config::PromptTier::Full,
        true, // sovereign (test mock)
    );
    dog_contract(&dog).await;
}
