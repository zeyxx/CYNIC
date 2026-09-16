//! Pure Layer 2 State Channels & Task HTLCs for CYNIC Multi-Agent Systems
//!
//! References:
//! - RFC-002: CYNIC 5-Layer Decentralized Architecture (Layer 2)
//! - Bitcoin Lightning Network (Poon & Dryja 2016)
//! - Multi-Party Virtual State Channels (Dziembowski et al., EUROCRYPT 2019)
//! - BitVM & Optimistic Computation Verification (Robin Linus 2023)
//!
//! 100% Zero-I/O Domain Core: Pure mathematical state machines, sequencing,
//! and cryptographic commitment verification. Zero sockets, zero disk I/O.

pub mod channel;
pub mod htlc;
pub mod types;

#[cfg(test)]
mod tests;

pub use channel::AgentChannel;
pub use htlc::TaskHtlc;
pub use types::{
    AGENT_CHANNEL_SCHEMA, ChannelPhase, ChannelSettlementProof, SETTLEMENT_PROOF_SCHEMA,
    TASK_HTLC_SCHEMA,
};
