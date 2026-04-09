//! `sources` — inbound observation source implementations.
//!
//! This module hosts adapters for external systems that feed events into the
//! kernel's observation pipeline. Each source implements the `ObservationSource`
//! trait (see `crate::domain::source`).
//!
//! Architecture absorbed from prior art (research 2026-04-09):
//! - Vtable Channel + ring-buffer bus (nullclaw, Zig)
//! - Source-as-async-function + delta engine (Crucix, JS)
//! - Adapter interface + Message/Thread (vercel/chat, TS)
//! - Dual-interface boundary + context callback bundle (pi-mono, TS)
//!
//! Lifecycle: sources are spawned at kernel boot via the supervisor (TODO).
//! Each source runs in its own `tokio::spawn` task with exponential backoff
//! restart on failure, kill-switch file check, and structured audit logging.
//!
//! Security: sources holding credentials use `CredentialPort` (TODO) for
//! encrypted-at-rest secrets. The first paranoid implementor (Telegram USER mode)
//! uses GPG encryption + ramfs runtime decryption.
//!
//! Implementors planned:
//! - `x_observer` — Playwright-driven scrape of x.com via the user's PWA session
//! - `telegram_user` — MTProto USER mode via Python Telethon sidecar (`tokio::process`)
//! - `slack_bridge` — wraps existing slack MCP, mirrors messages into observations
//!
//! All implementors deferred to subsequent commits — this commit establishes
//! the module home only.

// Re-export the trait surface so consumers can `use crate::sources::ObservationSource`.
pub use crate::domain::source::{
    MessageRef, ObservationSource, OutboundSource, SourceEmitter, SourceError, SourceEvent,
    SourceHealth,
};
