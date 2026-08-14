//! Webhook handlers — push perception from external sources (GitHub, etc.)
//!
//! Pattern: external push → kernel validation → dispatch agent task → executor acts

pub mod github;

pub use self::github::github_release_handler;
