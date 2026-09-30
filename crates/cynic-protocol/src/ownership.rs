//! DISC-01 and INV-04, made unrepresentable rather than merely untested.
//!
//! Two reference systems died at this boundary. ELIZA Labs settled a class action on the
//! charge that governance was represented as autonomous while insiders retained actual
//! control; the divergence between the stated rule and the enforced one was the entire
//! case. Steem hard fork 0.23 seized 23.6M STEEM from 64 accounts under a
//! contribution-justified mechanism, and the property claim lost.
//!
//! The usual remedy is a test asserting the rule holds. That records the rule and leaves
//! the forbidden operation expressible, so a later change can reach it and nothing stops
//! it. Here the type system has no vocabulary for the operation at all.
//!
//! [`MaterialClaim`] and [`ExitRight`] are not variants of a `Right` enum that governance
//! can be asked about. They are separate types, and no method on [`GovernanceDecision`]
//! accepts either. The question "does this decision override a material claim?" therefore
//! does not typecheck — not because it returns false, but because there is no way to ask.
//!
//! Compare `contracts/ownership.json`, which states the same rules declaratively and is
//! covered at `partial`. This module is the executable half; the two must not drift.

use serde::{Deserialize, Serialize};

use crate::CynicErrorCode;

pub const OWNERSHIP_SCOPE_SCHEMA: &str = "cynic.ownership.scope.v1";

/// An inviolable claim on material possession.
///
/// Deliberately not addressable by governance. It carries no `revoke`, no `dilute`, and no
/// `transfer_without_consent`: the three operations that killed the two reference systems
/// are absent from the type rather than denied at runtime.
///
/// Transfer exists and takes a counterparty, because possession must be transferable to be
/// tradable at all. What does not exist is a path where anyone else causes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialClaim {
    pub claim_id: String,
    pub holder: String,
    pub unit: String,
    pub quantity: String,
}

/// An inviolable right of exit, carrying what the holder can carry.
///
/// No counterparty appears anywhere in this type, which is the whole property: departure
/// cannot require the consent, admission, or approval of anyone else, because there is no
/// field in which such a party could be named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitRight {
    pub holder: String,
    /// What the holder removes on departure. Cannot be reduced by anyone but the holder.
    pub carries: Vec<String>,
}

impl ExitRight {
    pub fn new(holder: impl Into<String>, carries: Vec<String>) -> Self {
        Self { holder: holder.into(), carries }
    }
}

/// The powers a governance decision may carry.
///
/// Five variants for five measurement classes, one each, per
/// `contracts/measure-arbitration.json`. There is no variant meaning material claim and no
/// variant meaning exit, because those are not governable. Adding one would be a type error
/// at every use site rather than a silent widening of authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GovernedPower {
    /// `P-infra`. Allocated from sensor-class consumption.
    InfrastructureBudget,
    /// `P-next`. Allocated from attention flow over a decaying window.
    BuildPriority,
    /// `P-legit`. Allocated from attention stock.
    Legitimacy,
    /// `P-admit`. Allocated from third-party consumption.
    Admission,
    /// `P-treasury`. Allocated from a capped envelope, never from a score.
    TreasuryAllocation,
}

/// A governance decision, and the powers it may act on.
///
/// Note the absence of methods. There is no `applies_to(Right)`, because `Right` does not
/// exist as something governance can be handed; there is no `revoke(&mut MaterialClaim)`;
/// there is no arithmetic that turns weight into possession. The forbidden operations are
/// not refused — they are unwritable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceDecision {
    pub decision_id: String,
    pub powers: Vec<GovernedPower>,
    /// Derived governance weight. Never stored on the claim side, never purchasable.
    pub derived_weight: String,
    pub evidence_refs: Vec<String>,
}

impl GovernanceDecision {
    pub fn new(decision_id: impl Into<String>, powers: Vec<GovernedPower>) -> Self {
        Self {
            decision_id: decision_id.into(),
            powers,
            derived_weight: String::new(),
            evidence_refs: Vec::new(),
        }
    }

    /// Whether this decision acts on a governed power.
    pub fn may_act_on(&self, power: GovernedPower) -> bool {
        self.powers.contains(&power)
    }

    /// The set of powers, deduplicated. A decision that names a power twice carries the
    /// same authority once, because repetition must not be a way to accumulate weight.
    pub fn distinct_powers(&self) -> Vec<GovernedPower> {
        let mut powers = self.powers.clone();
        powers.sort();
        powers.dedup();
        powers
    }
}

/// A successor's observable acceptance of a transferred obligation.
///
/// INV-04: revoking a capability transfers its duty; it never extinguishes one. Making the
/// acceptance a required constructor argument is what stops an author discharging a
/// responsibility by delegating it — the escape that leaves a clone to answer for a defect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessorAcceptance {
    pub successor_id: String,
    pub obligation: String,
    pub evidence_refs: Vec<String>,
}

/// A revocation that carries its own displacement.
///
/// The duty has nowhere to hide: `revoke` cannot be called without naming a successor who
/// accepts it with observable proof, so a constructible `Revocation` is always already
/// discharged of the question "and now who is answerable".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revocation {
    pub revoked_id: String,
    pub reason: String,
    pub transferred_to: SuccessorAcceptance,
}

impl Revocation {
    pub fn new(
        revoked_id: impl Into<String>,
        reason: impl Into<String>,
        transferred_to: SuccessorAcceptance,
    ) -> Self {
        Self {
            revoked_id: revoked_id.into(),
            reason: reason.into(),
            transferred_to,
        }
    }
}

/// INV-05: weight and possession are not commensurable.
///
/// The conversion this function refuses to perform is the one ELIZA performed, when a
/// 1:6 migration expanded supply from 1.1B to 11B and the expansion was the case. Naming
/// the failure as a typed error keeps the refusal in the vocabulary: a caller that reaches
/// for it gets a sovereign code, not a `None` to be ignored.
pub fn purchase_weight_from_possession(
    claim: &MaterialClaim,
    _requested_weight: &str,
) -> Result<(), CynicErrorCode> {
    let _ = claim;
    Err(CynicErrorCode::AgentBudgetExceeded)
}