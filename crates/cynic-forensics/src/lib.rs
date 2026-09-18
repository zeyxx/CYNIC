//! CYNIC Forensics Subsystem — Post-crash evidence collection and post-mortem reporting.

pub mod local;
pub mod port;
pub mod probe;
pub mod types;

pub use local::LocalForensicAdapter;
pub use port::{ForensicError, ForensicObservationPort};
pub use probe::{
    parse_boot_id, parse_boot_table_line, parse_crash_dump_header, parse_dmesg_gpu_fault,
    parse_hung_task, parse_kernel_trap, parse_rcu_context_switch_warning, parse_rcu_stall,
    parse_service_failed_line,
};
pub use types::{
    CrashKind, EvidenceSource, FORENSIC_TIMELINE_SCHEMA, ForensicEvidence, ForensicTimeline,
    POST_MORTEM_REPORT_SCHEMA, PostMortemReport, ServiceLoop, SocThresholds, TimelineEntry,
};
