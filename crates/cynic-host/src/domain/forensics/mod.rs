//! Forensics observation domain — post-crash evidence collection and post-mortem reporting.
//!
//! Pure parsers live in `probe`; the observation port is in `port`; the local
//! adapter that reads real host evidence is in `local`; canonical types are
//! re-exported from `cynic_protocol::forensics`.

pub mod local;
pub mod port;
pub mod probe;
pub mod types;

pub use local::LocalForensicAdapter;
pub use port::{ForensicError, ForensicObservationPort};
pub use probe::{
    parse_boot_id, parse_boot_table_line, parse_crash_dump_header, parse_dmesg_gpu_fault,
    parse_kernel_trap, parse_service_failed_line,
};
pub use types::{
    CrashKind, EvidenceSource, ForensicEvidence, ForensicTimeline, PostMortemReport, ServiceLoop,
    SocThresholds, TimelineEntry, FORENSIC_TIMELINE_SCHEMA, POST_MORTEM_REPORT_SCHEMA,
};