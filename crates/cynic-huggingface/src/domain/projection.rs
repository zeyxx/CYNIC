use serde::{Deserialize, Serialize};

/// Projection layer cannot be exported to an external provider until it passes
/// this observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectionReadiness {
    pub has_redaction: bool,
    pub has_manifest: bool,
    pub canonical_bytes: u64,
}

impl ProjectionReadiness {
    pub fn export_ready(&self) -> bool {
        self.has_redaction && self.has_manifest && self.canonical_bytes > 0
    }
}