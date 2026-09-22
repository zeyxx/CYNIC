//! In-memory test adapters for runner tests (Phase 4).
//!
//! Provider HTTP adapters live in separate crates:
//! - `cynic-domain-porkbun` — RegistrarPort impl
//! - `cynic-domain-cloudflare` — CloudflareDnsPort + EdgeProbePort impl
//! - `cynic-domain-askesis` — BackupArchiver
//!
//! This memory adapter provides deterministic, zero-I/O implementations
//! of all four ports for unit-testing the runner's orchestration logic.

#[cfg(test)]
pub mod memory;
