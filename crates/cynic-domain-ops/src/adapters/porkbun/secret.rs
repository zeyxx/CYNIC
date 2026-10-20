//! Secret resolution re-export — V2 migration.
//!
//! `EnvSecretProvider` is now a type alias for `ProcessEnvironmentSecretProvider`
//! from the shared `cynic-secrets` crate (horizontal secret management layer).
//!
//! V2 path: use `CompositeSecretProvider` (env → file → keyring) instead.
//! The local `EnvSecretProvider` alias is retained for backward compatibility
//! with `cynic-domain-porkbun/src/adapter.rs` and `cynic-cli`.

pub use cynic_secrets::ProcessEnvironmentSecretProvider as EnvSecretProvider;
