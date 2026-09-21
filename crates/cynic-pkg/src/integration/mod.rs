//! Sovereign ecosystem integrations domain.
//!
//! Provides lifecycle models, provider specifications, catalog definitions, and
//! state reconciliation logic.

pub mod catalog;
pub mod decision;
pub mod lifecycle;
pub mod types;

pub use catalog::integration_specs;
pub use decision::{
    decide_integration_action, IntegrationDecision, IntegrationDecisionSurface,
    IntegrationPlanAction,
};
pub use lifecycle::{
    BoundaryKind, LifecycleDisposition, LifecycleProjection, LifecycleStatus, SurfaceStatus,
};
pub use types::{
    ConnectionState, IntegrationCapability, IntegrationConnectionObservation,
    IntegrationDependencySpec, IntegrationDependencyView, IntegrationKind, IntegrationObservation,
    IntegrationObservationContext, IntegrationPrincipal, IntegrationSpec, IntegrationStatusSummary,
    IntegrationToolingAuthorityView, IntegrationsStatusReport, RawIntegrationObservation,
};
