//! Shared vocabulary re-exported for the forensics domain.
//!
//! Pure contract types live in `cynic-protocol/src/forensics.rs` so that the
//! kernel, node, host, and CLI surfaces all reference one canonical definition
//! with never a circular dependency. This file is a thin re-export shim.

pub use cynic_protocol::forensics::{
    CrashKind, EvidenceSource, FORENSIC_TIMELINE_SCHEMA, ForensicEvidence, ForensicTimeline,
    POST_MORTEM_REPORT_SCHEMA, PostMortemReport, ServiceLoop, SocThresholds, TimelineEntry,
};