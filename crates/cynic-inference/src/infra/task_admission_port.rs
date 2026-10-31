//! Admission port for the DomainTaskQueue (plan §10 step 3).
//!
//! Canonical home is `cynic_protocol::task_admission`; this module
//! re-exports for backward compatibility.

pub use cynic_protocol::task_admission::{TaskAdmissionPort, TaskAdmissionError};
