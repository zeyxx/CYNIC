//! CYNIC Autoresearcher — trace observation, pattern matching, and skill synthesis.
//!
//! Trace observation (`TraceFsAdapter`) and error bucket signatures
//! (`bucket_signatures`) for autonomous skill synthesis and distillation.

pub mod bucket_signatures;
pub mod trace_fs;

pub use bucket_signatures::{
    BucketSignature, bucket_domains, first_matching_bucket, matching_buckets, signature_count,
};
pub use trace_fs::{
    TraceFsAdapter, TraceSource, TraceUnitEmitter, default_bucket_patterns, observe_reader,
};
