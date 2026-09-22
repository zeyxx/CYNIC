//! Canonical configuration façade for the kernel infra layer.
//!
//! This module intentionally groups configuration by role:
//! - `model`: domain value objects and enums.
//! - `loading`: TOML/env-backed loaders and project fallback rules.
//!
//! Keep command surfaces here thin: no parsing policy is implemented in callers.

pub mod loading;
pub mod model;

pub use loading::{
    derive_slots_url, load_backends, load_backends_from_env, load_organ_remediations,
};
pub use model::{
    AuthStyle, BackendConfig, BackendRemediation, BackendType, CircuitBreakerConfig,
    ConvergenceConfig, DogDefaults, DogThreshold, DogThresholds, ErrorDetection, KScoreConfig,
    MonitoringConfig, OrganRemediation, PromptTier, StorageConfig, StorageIdentityConfig,
    VerdictSettings,
};
