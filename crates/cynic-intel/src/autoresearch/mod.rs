//! CYNIC Autoresearcher — trace observation, pattern matching, and reflection hypotheses.
//!
//! Trace observation (`TraceFsAdapter`), error bucket signatures (`bucket_signatures`),
//! and hypothesis generation (`reflection`) for autonomous skill synthesis and distillation.

pub mod bucket_signatures;
pub mod reflection;
pub mod trace_fs;

pub use bucket_signatures::{
    bucket_domains, first_matching_bucket, matching_buckets, signature_count, BucketSignature,
};
pub use reflection::{parse_hypotheses, OfflineReflection, ReflectionAdapter};
pub use trace_fs::{default_bucket_patterns, observe_reader, TraceFsAdapter, TraceSource, TraceUnitEmitter};
