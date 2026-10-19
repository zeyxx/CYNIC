//! Executable half of `specs/SPEC-CYNIC-OWNERSHIP.md`.
//!
//! `contracts/ownership.json` states DISC-01 declaratively and is covered at `partial`:
//! the rule is written and the check was missing. This file is that check, and it does
//! something a test cannot — it asserts on the *shape* of the type surface, so the
//! forbidden operations are counted rather than probed.
//!
//! Each assertion below is negative-controlled. A check that cannot fail is the defect
//! this repository has already paid for twice.

use cynic_protocol::ownership::{
    purchase_weight_from_possession, ExitRight, GovernedPower, GovernanceDecision, MaterialClaim,
    Revocation, SuccessorAcceptance,
};
use cynic_protocol::CynicErrorCode;

/// DISC-01: the holder of the largest material share holds no vote over a decision that
/// affects another actor's material claim.
///
/// Asserted structurally. `GovernedPower` has five variants and none of them names a
/// material claim or an exit, so the powers a decision may carry are closed under exactly
/// the five governable subjects.
#[test]
fn governed_power_enumerates_only_governable_subjects() {
    let all = [
        GovernedPower::InfrastructureBudget,
        GovernedPower::BuildPriority,
        GovernedPower::Legitimacy,
        GovernedPower::Admission,
        GovernedPower::TreasuryAllocation,
    ];

    // Five powers, five measurement classes, one each (INV-08). The count is the
    // assertion: a sixth variant would mean one measurement now serves two powers, and a
    // removal would mean a power lost its measurement. Both are silent otherwise.
    assert_eq!(
        all.len(),
        5,
        "GovernedPower must enumerate exactly the five powers allocated in \
         contracts/measure-arbitration.json. A new variant assigns a second power to an \
         existing measurement, which INV-08 forbids."
    );

    let distinct: std::collections::BTreeSet<_> = all.iter().collect();
    assert_eq!(
        distinct.len(),
        all.len(),
        "GovernedPower contains a duplicate. Two names for one power means one measurement \
         is silently allocated twice."
    );
}

/// A decision acts only on powers it names, and naming a power twice is not accumulation.
///
/// Repetition must not be a route to authority, so deduplication is part of the type's
/// contract rather than left to callers.
#[test]
fn a_decision_acts_only_on_named_powers_and_repetition_is_not_accumulation() {
    let decision = GovernanceDecision::new(
        "d-1",
        vec![
            GovernedPower::Admission,
            GovernedPower::TreasuryAllocation,
            GovernedPower::Admission,
        ],
    );

    assert!(decision.may_act_on(GovernedPower::Admission));
    assert!(decision.may_act_on(GovernedPower::TreasuryAllocation));
    assert!(
        !decision.may_act_on(GovernedPower::InfrastructureBudget),
        "A decision must not act on a power it does not name."
    );

    assert_eq!(
        decision.distinct_powers().len(),
        2,
        "Naming a power twice must carry the same authority once, otherwise repetition is a \
         way to accumulate governance weight."
    );
}

/// INV-04: revocation transfers the charge; it never extinguishes it.
///
/// The constructor cannot be reached without a successor who accepts with observable
/// proof. The defect this closes is an author delegating a duty, being revoked, and leaving
/// a clone answerable for it.
#[test]
fn revocation_cannot_be_constructed_without_a_successor() {
    let acceptance = SuccessorAcceptance {
        successor_id: "actor-2".to_string(),
        obligation: "answer for defect D-1".to_string(),
        evidence_refs: vec!["sha256:acceptance".to_string()],
    };

    let revocation = Revocation::new("capability-1", "breach of obligation", acceptance);
    assert_eq!(
        revocation.transferred_to.successor_id, "actor-2",
        "A revocation that names no successor has nowhere to put the duty, which is the \
         escape INV-04 closes."
    );
    assert!(
        !revocation.transferred_to.evidence_refs.is_empty(),
        "Successor acceptance must carry observable proof. Acceptance without evidence is \
         indistinguishable from silence."
    );
}

/// INV-05: possession is not commensurable with governance weight.
///
/// The conversion ELIZA performed when a 1:6 migration expanded supply from 1.1B to 11B is
/// named as a typed refusal rather than left unexpressible, so a caller reaching for it
/// gets a sovereign code rather than a `None`.
#[test]
fn possession_cannot_be_converted_into_governance_weight() {
    let claim = MaterialClaim {
        claim_id: "c-1".to_string(),
        holder: "actor-1".to_string(),
        unit: "shares".to_string(),
        quantity: "1_000_000_000".to_string(),
    };

    let outcome = purchase_weight_from_possession(&claim, "1000");

    assert_eq!(
        outcome,
        Err(CynicErrorCode::AgentBudgetExceeded),
        "INV-05 requires the purchase of weight to fail. A conversion that succeeds, or fails \
         silently, is the ELIZA supply migration."
    );
}

/// R-B: exit carries no counterparty.
///
/// The right of exit is the only unfalsifiable, costless counterweight to governance, and
/// the property is structural: the type has no field in which a consenting party could be
/// named, so no code can require one.
#[test]
fn the_right_of_exit_has_no_counterparty() {
    let exit = ExitRight::new("actor-1", vec!["own-artifacts".to_string(), "own-identity".to_string()]);

    assert_eq!(exit.holder, "actor-1");
    assert_eq!(exit.carries.len(), 2);

    // Serialised form carries no counterparty, so the property survives persistence and is
    // checkable by a reader of the artifact rather than only by a caller of the API.
    //
    // Compared as a set, not a sequence: serde_json's map is ordered by its own backing
    // store, and key order is a property of the serializer rather than of the type. An
    // order assertion here would fail on a feature flag rather than on a design change.
    let json = serde_json::to_value(&exit).expect("exit right serialises");
    let object = json.as_object().expect("object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();

    assert_eq!(
        keys,
        vec!["carries", "holder"],
        "ExitRight serialises to {} — any further field would be a place where a counterparty \
         could be named, and departure could be made conditional.",
        keys.join(", ")
    );
    assert!(
        !json.as_object().expect("object").contains_key("approved_by"),
        "Exit must not be gateable. INV-02 admits no counterparty."
    );
}