#![forbid(unsafe_code)]

//! cynic-domain-porkbun — Porkbun API adapter for cynic-domain-ops.
//!
//! Implements `RegistrarPort` against the Porkbun API v2 (registrar-level
//! operations: availability, registration, nameserver management, DNS read).
//! Also provides `EnvSecretProvider` — a `SecretProvider` port implementation
//! that resolves `SecretReference` from process environment variables.
//!
//! Credentials are NEVER inline. The API key and secret are resolved through
//! `SecretReference` (provider = `process_environment`, locator = env var name).
//! The `PorkbunRegistrar` struct holds resolved strings received from the
//! `SecretProvider` at construction time.
//!
//! Authority: infra/registry.json (porkbun provider declaration),
//! foundation.json §37 (portable_host_contract — HTTP behind a port).

pub mod adapter;
pub mod error;
pub mod secret;

pub use adapter::PorkbunRegistrar;
pub use error::PorkbunError;
pub use secret::EnvSecretProvider;
