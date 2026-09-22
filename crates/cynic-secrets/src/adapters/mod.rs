//! Concrete secret adapter implementations.
//!
//! Each adapter implements `SecretProviderPort` (and optionally `SecretStorePort`).
//! Adapters perform I/O; the protocol and ports layers remain pure.

pub mod env;
pub mod file;
pub mod keyring;
pub mod memory;

pub use env::ProcessEnvironmentSecretProvider;
pub use file::EncryptedFileSecretProvider;
pub use keyring::OsCredentialStoreSecretProvider;
pub use memory::MemorySecretProvider;
