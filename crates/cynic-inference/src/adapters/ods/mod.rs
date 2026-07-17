//! Integration boundary with **[Osmantic ODS](https://github.com/Osmantic/ODS)**.
//!
//! ## Authority split
//!
//! | Layer | Owner | Role |
//! |---|---|---|
//! | **CYNIC native** | this crate (`adapters::local`, routing SSOT) | Organism inference law: vault, profiles, start/stop, sovereign routing |
//! | **ODS** | [Osmantic/ODS](https://github.com/Osmantic/ODS) (external product) | Maintained framework to **bootstrap** a full local AI environment (llama-server, Open WebUI, dashboard, agents, …) |
//! | **This adapter** | CYNIC | Observe ODS-compatible endpoints and publish them into CYNIC routing — **does not** reimplement or replace ODS |
//!
//! CYNIC builds its own system. ODS is not a CYNIC subsystem: it is a first-class
//! external bootstrap/runtime for the host AI stack, maintained upstream, that
//! CYNIC may consume when present.
//!
//! ## What this module does today
//!
//! Probe common local OpenAI-compatible listeners (including those ODS exposes,
//! e.g. host `11434` / `8080`) and optionally merge models into `inference.toml`.
//!
//! ## What this module does **not** do
//!
//! - install or upgrade ODS
//! - own ODS model catalog / `ods model swap` / dashboard lifecycle
//! - replace native vault + `llama-server@` control
//!
//! Native control: `adapters::local`.
//! Historical entrypoint: `crate::ods_discovery` (prefer this module for new code).

pub use crate::ods_discovery::{
    DiscoveredService, DiscoveryReport, discover_ods_services, sync_discovered_backends,
    run_sync_pipeline,
};

/// Surface label: CYNIC integration with Osmantic ODS (or compatible peers).
pub const ODS_SURFACE: &str = "ods_integration";

/// Upstream product identity (not a CYNIC crate).
pub const ODS_UPSTREAM: &str = "https://github.com/Osmantic/ODS";

/// Schema id for ODS integration discovery reports.
pub const ODS_DISCOVERY_SCHEMA: &str = "cynic.inference.ods_discovery.v1";

/// Host API ports commonly used by ODS (see upstream README).
/// Linux Docker host often maps llama-server to **11434**; native paths use **8080**.
pub const ODS_DEFAULT_HOST_PORTS: &[u16] = &[11434, 8080];
