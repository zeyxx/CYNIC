//! CYNIC Autoresearcher — trace observation, pattern matching, and reflection hypotheses.
//!
//! Trace observation (`TraceFsAdapter`), error bucket signatures (`bucket_signatures`),
//! and hypothesis generation (`reflection`) for autonomous skill synthesis and distillation.

pub mod bucket_signatures;
pub mod reflection;
pub mod trace_fs;

pub use bucket_signatures::{
    BucketSignature, bucket_domains, first_matching_bucket, matching_buckets, signature_count,
};
pub use reflection::{OfflineReflection, ReflectionAdapter, parse_hypotheses};
pub use trace_fs::{
    TraceFsAdapter, TraceSource, TraceUnitEmitter, default_bucket_patterns, observe_reader,
};
