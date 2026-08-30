//! Read-only observation port for the forensics vertical.
//!
//! Port = pure trait naming WHAT can be observed, never HOW (that is the
//! adapter's job). Adapters (`LocalForensicAdapter`) implement this and are
//! injected by the CLI/host surface.

use crate::domain::forensics::types::{ForensicEvidence, ForensicTimeline, PostMortemReport, ServiceLoop};

pub trait ForensicObservationPort {
    /// Collect all forensic evidence for a given boot.
    fn collect_evidence(&self, boot_id: &str) -> Result<Vec<ForensicEvidence>, ForensicError>;

    /// Build a forensic timeline for a given boot.
    fn timeline(&self, boot_id: &str) -> Result<ForensicTimeline, ForensicError>;

    /// Produce a post-mortem report for a given boot.
    fn post_mortem(&self, boot_id: &str) -> Result<PostMortemReport, ForensicError>;

    /// List service restart loops observed on the host.
    fn service_loops(&self) -> Result<Vec<ServiceLoop>, ForensicError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ForensicError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("command failed: {0}")]
    Command(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
}