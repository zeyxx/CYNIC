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
//! Authority: `cynic-protocol::secret_use` — the purpose/class matrix and the
//! role match are a Rust type, not a table. `secret_reference.rs` defines the
//! canonical reference types; this crate consumes them without duplication.

pub mod adapters;
pub mod composite;
pub mod error;
pub mod identity;
pub mod ports;
pub mod protocol;
pub mod types;

// Re-export canonical secret types from cynic-protocol — no duplication.
pub use cynic_protocol::secret_reference::{
    SecretClass, SecretProvider as SecretProviderKind, SecretReference,
};
// Re-export the secret-use governance types from cynic-protocol.
pub use cynic_protocol::secret_use;
// Re-export the port traits and concrete providers from this crate.
pub use adapters::{
    EncryptedFileSecretProvider, OsCredentialStoreSecretProvider, ProcessEnvironmentSecretProvider,
};
pub use composite::CompositeSecretProvider;
pub use ports::{SecretProviderPort, SecretStorePort, SecretUseGatePort};
pub use protocol::{
    SECRET_USE_EXECUTION_SCHEMA, SecretEntry, SecretError, SecretStatus, SecretUseExecutionReport,
    SecretUseHandle, SecretUseOutcome,
};
// Re-export identity types (merged from cynic-identity).
pub use error::IdentityError;
pub use types::{NodeId, NodeIdentity, SigningMaterial};

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

/// Universal resolution helper — resolves a secret through the full composite
/// hierarchy: `ProcessEnvironment` -> `PrincipalOwnedFile` (~/.config/cynic/secrets
/// and /etc/cynic/secrets) -> `OsCredentialStore`.
///
/// Use this everywhere secret material is needed without coupling to a specific storage backend.
pub fn resolve_secret(id: &str, env_var_name: &str) -> Option<String> {
    let reference = SecretReference::env(id, env_var_name);
    CompositeSecretProvider::default().resolve(&reference)
}
