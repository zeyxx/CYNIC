//! Concrete adapters for cynic-release ports.
//!
//! Side-effecting implementations (cargo check, secret lint, git actuation)
//! live here, never in the pure domain core (`types`, `contracts`, `evaluator`).

pub mod release_gate_adapter;
pub mod release_relay_adapter;
pub mod secret_lint;
