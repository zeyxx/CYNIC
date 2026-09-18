//! Pure types for CYNIC release decision engine and relay projection.

use serde::{Deserialize, Serialize};

/// SemVer representation with optional hybrid date suffix (conforming to contracts/release-versioning.json).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ReleaseVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub date_suffix: Option<String>,
}

impl ReleaseVersion {
    pub fn new(major: u64, minor: u64, patch: u64, date_suffix: Option<String>) -> Self {
        Self {
            major,
            minor,
            patch,
            date_suffix,
        }
    }

    /// Format canonical tag string (e.g. `v0.2.1` or `v0.2.1-2026.08.19`).
    pub fn to_tag(&self) -> String {
        match &self.date_suffix {
            Some(date) => format!("v{}.{}.{}-{}", self.major, self.minor, self.patch, date),
            None => format!("v{}.{}.{}", self.major, self.minor, self.patch),
        }
    }

    /// Parse a SemVer tag (e.g. `v0.2.1` or `0.2.1`).
    pub fn parse_tag(tag: &str) -> Option<Self> {
        let tag = tag.trim().strip_prefix('v').unwrap_or(tag.trim());
        let (num_part, date_part) = match tag.find('-') {
            Some(idx) => (&tag[..idx], Some(tag[idx + 1..].to_string())),
            None => (tag, None),
        };

        let parts: Vec<&str> = num_part.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;

        Some(Self {
            major,
            minor,
            patch,
            date_suffix: date_part,
        })
    }

    /// Determine whether `self` is strictly monotonic after `prior`.
    pub fn is_strictly_monotonic_after(&self, prior: &ReleaseVersion) -> bool {
        if self.major > prior.major {
            return true;
        }
        if self.major == prior.major && self.minor > prior.minor {
            return true;
        }
        if self.major == prior.major && self.minor == prior.minor && self.patch > prior.patch {
            return true;
        }
        false
    }
}

/// Release candidate proposal submitted for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCandidate {
    pub commit_sha: String,
    pub branch: String,
    pub proposed_version: ReleaseVersion,
    pub commit_messages: Vec<String>,
}

/// Individual gate evidence record provided by execution layers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateProofRecord {
    pub name: String,
    pub passed: bool,
    pub detail: String,
    pub observed_at_unix_s: u64,
}

/// Consolidated gate evaluation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEvaluationReport {
    pub schema: &'static str,
    pub all_passed: bool,
    pub records: Vec<GateProofRecord>,
    pub blockers: Vec<String>,
}

/// Final verdict on whether a release candidate is eligible.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReleaseEligibilityVerdict {
    Eligible {
        schema: &'static str,
        version: ReleaseVersion,
        commit_sha: String,
        tag_name: String,
        decided_at_unix_s: u64,
    },
    Blocked {
        schema: &'static str,
        reasons: Vec<String>,
        decided_at_unix_s: u64,
    },
}

impl ReleaseEligibilityVerdict {
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible { .. })
    }
}

/// Nature of relay action to be performed by VCS runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelayActionKind {
    TagPush,
    MainFastForward,
    MainForcedPromotion,
}

/// Immutable proof and authorization decision for public relay projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RelayDecisionProof {
    Authorized {
        schema: &'static str,
        sovereign_commit: String,
        target_remote: String,
        target_ref: String,
        action_kind: RelayActionKind,
        decided_at_unix_s: u64,
    },
    Rejected {
        schema: &'static str,
        blockers: Vec<String>,
        decided_at_unix_s: u64,
    },
}

impl RelayDecisionProof {
    pub fn is_authorized(&self) -> bool {
        matches!(self, Self::Authorized { .. })
    }
}
