//! Error Classification — categorizes failures into quota/transient/critical.
//! Uses error_detection patterns from backends.toml to determine recovery strategy.
//!
//! K15 consumer: Routes errors to appropriate handlers:
//! - Quota exhaustion: skip Dog gracefully, no circuit open
//! - Transient: retry with backoff, don't count as permanent failure
//! - Critical: open circuit, alert operators

use crate::domain::health_gate::FailureReason;
use crate::infra::config::ErrorDetection;

/// Classifies an error into quota/transient/critical for handling decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Quota exhausted (rate limit, capacity limit). Skip Dog gracefully.
    Quota,
    /// Temporary network/service issue (timeout, temporarily unavailable). Retry.
    Transient,
    /// Permanent failure (subprocess not found, permission denied). Circuit open.
    Critical,
    /// Unknown error. Conservative: treat as critical.
    Unknown,
}

impl ErrorCategory {
    /// Classify an error message using patterns from error_detection config.
    /// Pattern matching is case-insensitive and looks for substring matches.
    pub fn classify(error_msg: &str, detection: &ErrorDetection) -> Self {
        let msg_lower = error_msg.to_lowercase();

        // Check quota patterns first (highest precedence in recovery strategy)
        for pattern in &detection.quota_patterns {
            if msg_lower.contains(&pattern.to_lowercase()) {
                return Self::Quota;
            }
        }

        // Check transient patterns
        for pattern in &detection.transient_patterns {
            if msg_lower.contains(&pattern.to_lowercase()) {
                return Self::Transient;
            }
        }

        // Check critical patterns
        for pattern in &detection.critical_patterns {
            if msg_lower.contains(&pattern.to_lowercase()) {
                return Self::Critical;
            }
        }

        // No match: conservative default
        Self::Unknown
    }

    /// Should the circuit breaker open on this error?
    /// Quota errors skip gracefully without opening circuit.
    /// Transient errors don't open circuit.
    /// Critical and unknown errors open circuit.
    pub fn should_open_circuit(self) -> bool {
        matches!(self, Self::Critical | Self::Unknown)
    }

    /// Should we retry on this error?
    /// Transient errors retry. Quota errors skip (not a retry case).
    /// Critical errors don't retry.
    pub fn should_retry(self) -> bool {
        self == Self::Transient
    }

    /// Should we skip this Dog (don't count as failure)?
    /// Quota errors are skipped silently — resource constraint, not Dog failure.
    pub fn should_skip(self) -> bool {
        self == Self::Quota
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quota => write!(f, "quota_exhaustion"),
            Self::Transient => write!(f, "transient_error"),
            Self::Critical => write!(f, "critical_error"),
            Self::Unknown => write!(f, "unknown_error"),
        }
    }
}

/// Classify a FailureReason into an error category.
pub fn classify_failure(reason: &FailureReason, detection: &ErrorDetection) -> ErrorCategory {
    let msg = reason.to_string();
    ErrorCategory::classify(&msg, detection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_classification() {
        let detection = ErrorDetection::default();
        let category = ErrorCategory::classify(
            "TerminalQuotaError: You have exhausted your capacity",
            &detection,
        );
        assert_eq!(category, ErrorCategory::Quota);
        assert!(category.should_skip());
        assert!(!category.should_open_circuit());
    }

    #[test]
    fn test_transient_classification() {
        let detection = ErrorDetection::default();
        let category =
            ErrorCategory::classify("The service is temporarily unavailable", &detection);
        assert_eq!(category, ErrorCategory::Transient);
        assert!(category.should_retry());
        assert!(!category.should_open_circuit());
    }

    #[test]
    fn test_critical_classification() {
        let detection = ErrorDetection::default();
        let category = ErrorCategory::classify("gemini-cli: command not found: exit 1", &detection);
        assert_eq!(category, ErrorCategory::Critical);
        assert!(!category.should_retry());
        assert!(category.should_open_circuit());
    }

    #[test]
    fn test_case_insensitive_matching() {
        let detection = ErrorDetection::default();
        let category = ErrorCategory::classify("RATE LIMIT EXCEEDED", &detection);
        assert_eq!(category, ErrorCategory::Quota);
    }

    #[test]
    fn test_unknown_default() {
        let detection = ErrorDetection::default();
        let category = ErrorCategory::classify("some random error that doesn't match", &detection);
        assert_eq!(category, ErrorCategory::Unknown);
        assert!(category.should_open_circuit());
    }
}
