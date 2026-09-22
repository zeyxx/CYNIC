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
pub use adapters::ProcessEnvironmentSecretProvider;
pub use composite::CompositeSecretProvider;
pub use ports::{SecretProviderPort, SecretStorePort};
pub use protocol::{SecretEntry, SecretError, SecretStatus};

/// V1 resolution helper — resolves an env var through the standard provider.
///
/// Atomic migration path replacing `std::env::var("X").ok()`:
/// ```rust
/// // BEFORE: let key = std::env::var("MY_KEY").ok();
/// // AFTER:  let key = cynic_secrets::env_var("my-key", "MY_KEY");
/// ```
/// Future: `main.rs` injects `CompositeSecretProvider` for V2→V3 fallback;
/// this helper then delegates to the injected provider.
pub fn env_var(id: &str, var: &str) -> Option<String> {
    ProcessEnvironmentSecretProvider.resolve(&SecretReference::env(id, var))
}

/// V1 presence check — whether a secret env var is set.
///
/// Replaces `std::env::var("X").is_ok()` patterns.
pub fn env_var_present(id: &str, var: &str) -> bool {
    ProcessEnvironmentSecretProvider
        .status(&SecretReference::env(id, var))
        .present
}
