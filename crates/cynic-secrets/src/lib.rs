//! Horizontal secret management layer for all CYNIC crates.
//!
//! This crate provides:
//! - **Protocol** (pure types): `SecretStatus`, `SecretEntry`, `SecretError`
//! - **Ports** (traits): `SecretProviderPort` (read-only resolution),
//!   `SecretStorePort` (write/rotate/delete/list)
//! - **Composite provider**: tries `ProcessEnvironment` → `PrincipalOwnedFile`
//!   → `OsCredentialStore` in order
//! - **Adapters**: env (V1), encrypted file (V2), OS keyring (V3), memory (test)
//!
//! Authority: AGENTS.md secret classes table (HumanAdmin = bootstrap only).
//! `cynic-protocol::secret_reference::{SecretReference, SecretClass, SecretProvider}`
//! define the canonical types; this crate consumes them without duplication.

pub mod adapters;
pub mod composite;
pub mod ports;
pub mod protocol;

// Re-export canonical secret types from cynic-protocol — no duplication.
pub use cynic_protocol::secret_reference::{
    SecretClass, SecretProvider as SecretProviderKind, SecretReference,
};
// Re-export the port traits and concrete providers from this crate.
pub use composite::CompositeSecretProvider;
pub use ports::{SecretProviderPort, SecretStorePort};
pub use protocol::{SecretEntry, SecretError, SecretStatus};
