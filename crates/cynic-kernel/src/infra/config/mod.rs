//! Canonical configuration façade for the kernel infra layer.
//!
//! This module intentionally groups configuration by role:
//! - `model`: domain value objects and enums.
//! - `loading`: TOML/env-backed loaders and project fallback rules.
<<<<<<< HEAD
=======
//! - `prompts`: runtime prompt corpus loading.
>>>>>>> github/main
//!
//! Keep command surfaces here thin: no parsing policy is implemented in callers.

pub mod loading;
pub mod model;
<<<<<<< HEAD

pub use loading::{
    derive_slots_url, load_backends, load_backends_from_env, load_organ_remediations,
=======
pub mod prompts;

pub use loading::{
    derive_slots_url, load_backends, load_backends_from_env, load_dog_thresholds,
    load_organ_remediations, load_storage_config, load_system_contract,
>>>>>>> github/main
};
pub use model::{
    AuthStyle, BackendConfig, BackendRemediation, BackendType, CircuitBreakerConfig,
    ConvergenceConfig, DogDefaults, DogThreshold, DogThresholds, ErrorDetection, KScoreConfig,
<<<<<<< HEAD
    MonitoringConfig, OrganRemediation, PromptTier, StorageConfig, StorageIdentityConfig,
    VerdictSettings,
};
=======
    MonitoringConfig, OrganRemediation, PromptTier, StorageConfig, VerdictSettings,
};
pub use prompts::load_domain_prompts;
>>>>>>> github/main
