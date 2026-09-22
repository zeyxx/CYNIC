#![forbid(unsafe_code)]

//! cynic-domain-cloudflare — Cloudflare API adapter for cynic-domain-ops.
//!
//! Implements `CloudflareDnsPort` (zone CRUD, DNS record management) and
//! `EdgeProbePort` (authoritative NS queries via DNS-over-HTTPS, HTTP edge
//! verification with `--resolve` semantics).
//!
//! Credentials (Cloudflare API token) are injected at construction — resolved
//! from `SecretReference` via an injected `SecretProvider`. Never inline.
//!
//! Authority: infra/registry.json (cloudflare provider declaration),
//! foundation.json §37 (portable_host_contract — HTTP behind a port).

pub mod adapter;
pub mod error;

pub use adapter::CloudflareClient;
pub use error::CloudflareError;
