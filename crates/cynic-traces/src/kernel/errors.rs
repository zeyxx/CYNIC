//! Framework errors.
//!
//! Strict port of `apps/cynic-data/src/cynic_data/kernel/errors.py`.
//! `ContractViolation` lives in `crate::session` (the canonical-layer breach);
//! the remaining framework errors live here.

use std::fmt;

/// Base error for the data-centric framework.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSystemError(pub String);

impl fmt::Display for DataSystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data system error: {}", self.0)
    }
}

impl std::error::Error for DataSystemError {}

/// The requested capture record does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureNotFound(pub String);

impl fmt::Display for CaptureNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "capture not found: {}", self.0)
    }
}

impl std::error::Error for CaptureNotFound {}

/// The source is missing a table the contract declares as in-scope.
///
/// Fail-closed: an auxiliary read that cannot find its table must NOT degrade
/// to an empty value (which would silently drop session truth).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSchemaDrift(pub String);

impl fmt::Display for SourceSchemaDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source schema drift: {}", self.0)
    }
}

impl std::error::Error for SourceSchemaDrift {}
