use serde::{Deserialize, Serialize};

pub const CONTENT_ENVELOPE_SCHEMA: &str = "cynic.askesis.kyon_content_envelope.v1";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentReference {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentEnvelope {
    pub schema: String,
    pub content_ref: ContentReference,
    pub media_type: String,
    pub size_bytes: u64,
    pub digest: String,
    pub provenance: String,
    pub retention: String,
    pub raw_content_in_report: bool,
    pub evidence_refs: Vec<String>,
    pub residual_unknowns: Vec<String>,
}
