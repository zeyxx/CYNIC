//! Low-layer Solana observation contracts.
//!
//! This module deliberately does not own a Helius or ODYSSEUM implementation.
//! Providers and legacy warehouses are adapters around these portable shapes.

mod backfill;
mod backfill_record;
mod compatibility;
mod coverage;
mod coverage_report;
mod decode;
mod fanout;
mod fanout_plan;
mod fanout_record;
mod fanout_runner;
mod integrity;
mod live;
mod live_adapter;
mod metering;
mod provider_probe;
mod provider_registry;
mod pump;
mod reconcile;
mod storage;
mod trace;
mod types;

pub use backfill::{
    BackfillAction, BackfillCheckpoint, BackfillHaltReason, BackfillSpec, apply_history_page,
    next_backfill_action,
};
pub use backfill_record::{BackfillRecordError, BackfillRecordReceipt, record_history_page};
pub use compatibility::{
    CompatibilityError, LegacyObservation, MigrationRejection, MigrationReport,
    from_odysseum_value, migrate_odysseum_json_bytes, migrate_odysseum_json_directory,
    migrate_odysseum_json_file, read_json_lines,
};
pub use coverage::{CoverageGap, CoverageLedger, CoverageStatus};
pub use coverage_report::{
    CoverageReport, CoverageReportStatus, ObservationWindow, build_coverage_report,
    build_coverage_report_from_pages, read_coverage_report,
};
pub use decode::{
    DecodedInstruction, DecodedTransaction, InstructionDecodeStatus, classify_instruction,
    decode_transaction,
};
pub use fanout::{
    FanoutAction, FanoutCheckpoint, FanoutFailure, FanoutFailureKind, FanoutHaltReason,
    FanoutRetryDecision, FanoutRetryPolicy, FanoutSpec, apply_fanout_failure, apply_signature_page,
    apply_transaction, next_fanout_action,
};
pub use fanout_plan::{
    FanoutExecutionPlan, FanoutPlanError, FanoutPlanRequest, plan_fanout, plan_fanout_request,
};
pub use fanout_record::{
    FanoutRecordError, SignaturePageRecordReceipt, TransactionRecordReceipt, record_fanout_failure,
    record_signature_page, record_transaction,
};
pub use fanout_runner::{
    BoxSleepFuture, FanoutRunError, FanoutRunReport, FanoutSleeper, TokioFanoutSleeper, run_fanout,
};
pub use integrity::{IntegrityCheck, IntegrityStatus, verify_payload};
pub use live::{
    BoxSourceFuture, HistoryFilters, IndexedHistoryPage, IndexedHistoryQuery, IndexedHistorySource,
    IndexedTransactionRecord, RawTransactionObservation, SignaturePage, SignatureQuery,
    SignatureRecord, SignatureSource, SortOrder, SourceError, SourceRole, TransactionQuery,
    TransactionSource,
};
pub use live_adapter::JsonRpcAdapter;
pub use metering::{
    BackfillStrategy, CreditConfidence, CreditMeasurement, HeliusCreditCatalog, HeliusPlan,
    MetricsAggregate, ProjectUsageSnapshot, RequestMetrics, reconcile_project_usage,
    select_backfill_strategy,
};
pub use provider_probe::{
    ArchiveDepth, ProbeBudget, ProbeOutcome, ProbeSample, ProviderCapabilityProfile,
    ProviderProbeSpec, samples_from_signature_pages, samples_from_transactions,
    summarize_provider_probe,
};
pub use provider_registry::{
    ProbeAction, ProbeActionKind, ProbePlan, ProbePlanError, ProbeTarget, ProviderRegistry,
    ProviderRegistryEntry, build_probe_plan,
};
pub use pump::{
    InstructionFamily, PUMP_AMM_PROGRAM_ID, PUMP_FEES_PROGRAM_ID, PUMP_PROGRAM_ID, PumpAssetState,
    PumpLifecycle, PumpProfileRegistry, PumpProgramFamily, PumpProgramProfile,
};
pub use reconcile::{ReconciliationStatus, SignatureReconciliation, reconcile_signature_pages};
pub use storage::{
    BackfillStore, FanoutReadStore, FanoutStore, FileBackfillStore, FileFanoutStore, StorageError,
    StoredPageReceipt,
};
pub use trace::{ObservedTrace, TraceStatus, trace_legacy_json_bytes, trace_legacy_json_file};
pub use types::{Commitment, DecodeStatus, ProviderKind, SolanaObservation};
