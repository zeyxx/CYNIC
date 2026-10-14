//! Benchmark basket + profile forge: eval decisions over measured ledgers.
//!
//! The basket executor and the profile forge consume empirical records and
//! produce launch plans / vault profiles. Serving I/O (runtime start, vault
//! layout, env) is injected by the caller — never read here.

pub mod bench_basket_launcher;
pub mod forge;
pub mod retrieval_gain;

pub use bench_basket_launcher::{
    BasketItem, BasketItemResult, BasketLaunchReport, BasketPlan, BenchLaunchError,
    BenchLauncherPort, BenchStartReport, build_basket_from_ledger, launch_basket,
};
pub use forge::{
    FORGE_PROFILE_SCHEMA, ForgeProfileError, ForgeProfileReport, alias_from_artifact_id,
    forge_profile_from_benchmark_report, write_loopback_profile,
};
pub use retrieval_gain::{
    BenchConfig, BenchReport, FIXTURE_SCHEMA_V1, Fixture, GOLD_DIAG_DEPTH, LabeledQuery,
    QueryScores, REPORT_SCHEMA_V1, TemplatedEmbedder, TextPipeline, VectorCacheRecord, gold_rank,
    grade_hit, load_vector_cache, reciprocal_rank, run_bench, save_vector_cache, truncate_front,
};
