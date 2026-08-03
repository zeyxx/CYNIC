mod decision;
mod observer;
mod port;
mod record;
mod types;

pub use decision::decide_process_convergence;
pub use observer::LocalProcessConvergenceObserver;
pub use port::ProcessConvergenceObservationPort;
pub use record::record_process_convergence_epoch;
pub use types::{
    PROCESS_CONVERGENCE_DECISION_SCHEMA, PROCESS_CONVERGENCE_RECORD_SCHEMA,
    ProcessConvergenceDecision, ProcessConvergenceRecord, ProcessConvergenceRequest,
};

use crate::ports::evidence_store::ContentAddressedEvidenceStorePort;

pub fn observe_and_record_process_convergence_epoch(
    request: &ProcessConvergenceRequest,
    observer: &dyn ProcessConvergenceObservationPort,
    store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<ProcessConvergenceRecord, String> {
    let observed = observer.observe(&request.subject_id, &request.policy)?;
    let read_back = observer.observe(&request.subject_id, &request.policy)?;
    record_process_convergence_epoch(
        request,
        &observed,
        Some(&read_back),
        observer.producer_id(),
        store,
    )
}
