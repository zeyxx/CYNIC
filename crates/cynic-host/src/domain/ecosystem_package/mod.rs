//! Universal ecosystem package interface for operator tech-watch deps.
//!
//! Core packaging definitions, providers, and authority models are delegated to
//! the autonomous `cynic-pkg` micro-crate. Host-side integration adapters and
//! execution plans remain anchored to host execution boundaries.

pub use cynic_pkg::*;
pub use cynic_pkg::{
    authority, execution, layout, port, probe, providers, python_cli_policy, python_tooling,
    registry, tooling_authority, types, upstream_probe,
};

pub mod integration_apply;
pub mod integration_apply_types;
pub mod integration_catalog;
pub mod integration_connect;
pub mod integration_connect_types;
pub mod integration_decision;
pub mod integration_plan;
pub mod integration_plan_types;
pub mod integration_port;
pub mod integration_recipe;
pub mod integration_types;

// ── Stable public surface (CLI + tests) ─────────────────────────────────────

pub use integration_apply::apply_integration;
pub use integration_apply_types::IntegrationApplyReport;
pub use integration_catalog::integration_specs;
pub use integration_connect::connect_integration;
pub use integration_connect_types::IntegrationConnectReport;
pub use integration_decision::{
    decide_integration_action, IntegrationDecision, IntegrationDecisionSurface,
};
pub use integration_plan::{plan_integrations, INTEGRATIONS_PLAN_SCHEMA};
pub use integration_plan_types::{
    IntegrationExecutionPlan, IntegrationPlanAction, IntegrationPlanItem, IntegrationsPlanReport,
    IntegrationsPlanSummary,
};
pub use integration_port::{
    observe_integrations, IntegrationObservationPort, INTEGRATIONS_STATUS_SCHEMA,
};
pub use integration_recipe::{build_integration_recipe, IntegrationRecipe};
pub use integration_types::{
    ConnectionState, IntegrationCapability, IntegrationConnectionObservation,
    IntegrationDependencySpec, IntegrationDependencyView, IntegrationKind, IntegrationObservation,
    IntegrationObservationContext, IntegrationPrincipal, IntegrationSpec, IntegrationStatusSummary,
    IntegrationToolingAuthorityView, IntegrationsStatusReport, RawIntegrationObservation,
};
