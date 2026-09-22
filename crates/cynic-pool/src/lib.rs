//! cynic-pool: Contribution pool protocol for CYNIC.
//!
//! Coordinates agents (not nodes directly). Agents register, get difficulty targets,
//! submit verifiable contributions. Pool validates and aggregates into blocks.
//! Merkle proofs anchor cross-registry state.

pub mod difficulty;
pub mod error;
pub mod merkle;
pub mod protocol;
pub mod reward;
pub mod types;

pub use difficulty::PPLNSCalculator;
pub use difficulty::{Difficulty, adjust_difficulty, reward_for_block};
pub use error::PoolError;
pub use merkle::{MerkleProof, pool_merkle_proof, pool_merkle_root, verify_pool_proof};
pub use protocol::{PoolMessage, Task, TaskResult};
pub use reward::RewardState;
pub use types::{
    Block, BlockHeader, Contribution, ContributionKind, Contributor, Epoch, Role, Share,
    ShareProof, Worker,
};
