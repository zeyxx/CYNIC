//! Mesh subdomain (WireGuard overlay): entities/ports, plan, bundle, telemetry.
//!
//! FCIS: pure data models + trait ports in `orchestration`; side effects live in adapters.

pub mod bundle;
pub mod orchestration;
pub mod plan;
pub mod telemetry;

pub use bundle::*;
pub use orchestration::*;
pub use plan::*;
pub use telemetry::*;