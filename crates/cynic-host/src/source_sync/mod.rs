//! Source sync — observe → reconcile → apply across 3 layers.
//!
//! Cortex (worktree) → Forgejo (sovereign SSOT) → GitHub (public relay).
//!
//! Data-centric: every layer is pure data (types). Observation is read-only.
//! Reconcile is a pure function from observation to decision.
//! Apply is bounded mutation with rollback intent.
//!
//! Credentials are injected via `SourceSyncContext` — never read from env
//! by this module. The caller loads secrets from `/etc/cynic/secrets/`
//! using `SecretMaterial::read_file()` (see `cynic-host::secret_material`).

pub mod apply;
pub mod observe;
pub mod reconcile;
pub mod types;

pub use crate::credential::CredentialType;
pub use apply::{execute_plan, execute_plan_with_enrollment};
pub use observe::{observe_all_layers, observe_layer};
pub use reconcile::{plan, reconcile};
pub use types::*;
