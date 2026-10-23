//! cynic-vcs-providers — External VCS provider adapters for CYNIC sovereign VCS.

pub mod forgejo;
pub mod forgejo_token;
pub mod forgejo_webhook;
pub mod forgejo_webhook_server;
pub mod github;
pub mod pull_request;
pub mod registry_binding;
pub mod signed_exec;

pub use forgejo::ForgejoExternalSourceAdapter;
pub use forgejo_token::{
    BoxedForgejoTokenPort, ForgejoTokenApiPort, ForgejoTokenRuntimeSpec,
    LocalForgejoTokenApiAdapter,
};
pub use forgejo_webhook::{ForgejoPayload, extract_source_commit, validate_source_commit};
pub use forgejo_webhook_server::{WebhookServerConfig, spawn_webhook_server};
pub use github::GitHubExternalSourceAdapter;
pub use registry_binding::forgejo_pr_port_from_registry;
