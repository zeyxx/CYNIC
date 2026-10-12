use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalExportReport {
    pub source_content_hash: String,
    pub payload_hash: String,
    pub manifest_path: String,
    pub redacted_at_projection_layer: bool,
    pub bytes: u64,
}

impl ExternalExportReport {
    pub fn publisher_ready(&self) -> bool {
        !self.source_content_hash.is_empty()
            && !self.payload_hash.is_empty()
            && self.redacted_at_projection_layer
            && self.bytes > 0
    }
}