//! Concrete secret adapter implementations.
//!
//! Each adapter implements `SecretProviderPort` (and optionally `SecretStorePort`).
//! Adapters perform I/O; the protocol and ports layers remain pure.

pub mod env;
pub mod file;
pub mod keyring;
pub mod local_secret_use_gate;
pub mod memory;

pub use env::ProcessEnvironmentSecretProvider;
pub use file::EncryptedFileSecretProvider;
pub use keyring::OsCredentialStoreSecretProvider;
pub use local_secret_use_gate::{
    DEFAULT_HANDLE_TTL_MS, DEFAULT_INJECTION_ENV_VAR, LocalSecretUseGate, default_audit_path,
    resolve_via_composite,
};
pub use memory::MemorySecretProvider;
