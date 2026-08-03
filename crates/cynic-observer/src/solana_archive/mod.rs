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
    apply_history_page, next_backfill_action, BackfillAction, BackfillCheckpoint,
    BackfillHaltReason, BackfillSpec,
};
pub use backfill_record::{record_history_page, BackfillRecordError, BackfillRecordReceipt};
pub use compatibility::{
    from_odysseum_value, migrate_odysseum_json_bytes, migrate_odysseum_json_directory,
    migrate_odysseum_json_file, read_json_lines, CompatibilityError, LegacyObservation,
    MigrationRejection, MigrationReport,
};
pub use coverage::{CoverageGap, CoverageLedger, CoverageStatus};
pub use coverage_report::{
    build_coverage_report, build_coverage_report_from_pages, read_coverage_report, CoverageReport,
    CoverageReportStatus, ObservationWindow,
};
pub use decode::{
    classify_instruction, decode_transaction, DecodedInstruction, DecodedTransaction,
    InstructionDecodeStatus,
};
pub use fanout::{
    apply_fanout_failure, apply_signature_page, apply_transaction, next_fanout_action,
    FanoutAction, FanoutCheckpoint, FanoutFailure, FanoutFailureKind, FanoutHaltReason,
    FanoutRetryDecision, FanoutRetryPolicy, FanoutSpec,
};
pub use fanout_plan::{
    plan_fanout, plan_fanout_request, FanoutExecutionPlan, FanoutPlanError, FanoutPlanRequest,
};
pub use fanout_record::{
    record_fanout_failure, record_signature_page, record_transaction, FanoutRecordError,
    SignaturePageRecordReceipt, TransactionRecordReceipt,
};
pub use fanout_runner::{
    run_fanout, BoxSleepFuture, FanoutRunError, FanoutRunReport, FanoutSleeper, TokioFanoutSleeper,
};
pub use integrity::{verify_payload, IntegrityCheck, IntegrityStatus};
pub use live::{
    BoxSourceFuture, HistoryFilters, IndexedHistoryPage, IndexedHistoryQuery, IndexedHistorySource,
    IndexedTransactionRecord, RawTransactionObservation, SignaturePage, SignatureQuery,
    SignatureRecord, SignatureSource, SortOrder, SourceError, SourceRole, TransactionQuery,
    TransactionSource,
};
pub use live_adapter::JsonRpcAdapter;
pub use metering::{
    reconcile_project_usage, select_backfill_strategy, BackfillStrategy, CreditConfidence,
    CreditMeasurement, HeliusCreditCatalog, HeliusPlan, MetricsAggregate, ProjectUsageSnapshot,
    RequestMetrics,
};
pub use provider_probe::{
    samples_from_signature_pages, samples_from_transactions, summarize_provider_probe,
    ArchiveDepth, ProbeBudget, ProbeOutcome, ProbeSample, ProviderCapabilityProfile,
    ProviderProbeSpec,
};
pub use provider_registry::{
    build_probe_plan, ProbeAction, ProbeActionKind, ProbePlan, ProbePlanError, ProbeTarget,
    ProviderRegistry, ProviderRegistryEntry,
};
pub use pump::{
    InstructionFamily, PumpAssetState, PumpLifecycle, PumpProfileRegistry, PumpProgramFamily,
    PumpProgramProfile, PUMP_AMM_PROGRAM_ID, PUMP_FEES_PROGRAM_ID, PUMP_PROGRAM_ID,
};
pub use reconcile::{reconcile_signature_pages, ReconciliationStatus, SignatureReconciliation};
pub use storage::{
    BackfillStore, FanoutReadStore, FanoutStore, FileBackfillStore, FileFanoutStore, StorageError,
    StoredPageReceipt,
};
pub use trace::{trace_legacy_json_bytes, trace_legacy_json_file, ObservedTrace, TraceStatus};
pub use types::{Commitment, DecodeStatus, ProviderKind, SolanaObservation};
