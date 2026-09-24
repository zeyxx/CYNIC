//! cynic-vcs-forgejo — Forgejo API adapters for CYNIC sovereign VCS.

pub mod forgejo_token;
pub mod pull_request;
pub mod forgejo_webhook;
pub mod forgejo_webhook_server;

pub use forgejo_token::{
    BoxedForgejoTokenPort, ForgejoTokenApiPort, ForgejoTokenRuntimeSpec, LocalForgejoTokenApiAdapter,
};
pub use forgejo_webhook::{extract_source_commit, validate_source_commit, ForgejoPayload};
pub use forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};
