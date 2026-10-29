//! Unified autonomous agent trait.
//!
//! Every agent in `cynic-agent` implements [`AutonomousAgent`] alongside
//! its domain-specific port. The trait only names the agent and its
//! associated observation/decision/proof types; it authorizes nothing.

pub trait AutonomousAgent {
    type Observation;
    type Decision;
    type Proof;

    fn agent_name(&self) -> &'static str;
}