//! Compatibility adapter for deterministic wallet judgment — PRODUCT-DOMAIN QUARANTINE.
//!
//! Cynic-kernel keeps the invocation surface and target projection. The
//! deterministic scoring law lives in `cynic-intel`.
//! Canonical owner: `cynic-intel::wallet_judgment` (law). Track via
//! `cynic dev kernel-audit` area `product_domain_residual`. Not low-layer runtime law.

use crate::domain::dog::{AxiomScores, VerdictKind};

pub use cynic_intel::wallet_judgment::WalletProfile;

pub fn deterministic_dog(profile: &WalletProfile) -> (VerdictKind, AxiomScores) {
    let (kind, scores) = cynic_intel::wallet_judgment::deterministic_dog(profile);
    let verdict = match kind {
        cynic_intel::wallet_judgment::DeterministicDogKind::Howl => VerdictKind::Howl,
        cynic_intel::wallet_judgment::DeterministicDogKind::Wag => VerdictKind::Wag,
        cynic_intel::wallet_judgment::DeterministicDogKind::Growl => VerdictKind::Growl,
        cynic_intel::wallet_judgment::DeterministicDogKind::Bark => VerdictKind::Bark,
    };

    (
        verdict,
        AxiomScores {
            fidelity: scores.fidelity,
            phi: scores.phi,
            verify: scores.verify,
            culture: scores.culture,
            burn: scores.burn,
            sovereignty: scores.sovereignty,
            reasoning: Default::default(),
            ..Default::default()
        },
    )
}
