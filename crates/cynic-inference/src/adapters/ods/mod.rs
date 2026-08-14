//! Integration boundary with **OpenAI-compat peers** (optional
//! **[Osmantic ODS](https://github.com/Osmantic/ODS)** bootstrap).
//!
//! ## Authority split (loose coupling)
//!
//! | Layer | Owner | Role |
//! |---|---|---|
//! | **CYNIC native** | `adapters::local`, routing SSOT | vault, profiles, start/stop, sovereign routing |
//! | **Peer plane** | this adapter + `application::peer_attach` | probe `/v1` listeners; optional publish into SSOT |
//! | **ODS product** | [Osmantic/ODS](https://github.com/Osmantic/ODS) (external) | optional full-stack bootstrap (WebUI, agents, …) |
//! | **Install** | `cynic-host` ecosystem provider `ods` | clone pin + `install.sh` — **not** this crate |
//!
//! CYNIC does **not** own ODS lifecycle. Any OpenAI-compat peer works the same way.
//!
//! ## Operator surface
//!
//! ```text
//! cynic runtime inference-discover --json            # probe-only (default)
//! cynic runtime inference-discover --json --apply    # publish → inference.toml
//! cynic runtime inference-discover --json --apply --open-webui
//! cynic chat --ods                                   # prefer live peer for chat
//! cynic deps upgrade --yes --only ods                # optional product install
//! ```
//!
//! Prefer `crate::application::peer_attach` for new call sites (stable report).
//! Historical entrypoint: `crate::ods_discovery`.

pub use crate::application::peer_attach::{
    PeerAttachOptions, PeerAttachReport, peer_attach, peer_attach_apply, peer_attach_status,
};
pub use crate::ods_discovery::{
    DiscoveredService, DiscoveryReport, discover_ods_services, run_sync_pipeline,
    sync_discovered_backends,
};

/// Surface label: peer / ODS integration (not native vault).
pub const ODS_SURFACE: &str = "ods_integration";

/// Upstream product identity (hint only — not a hard dependency).
pub const ODS_UPSTREAM: &str = "https://github.com/Osmantic/ODS";

/// Schema id for peer attach / discovery reports.
pub const ODS_DISCOVERY_SCHEMA: &str = "cynic.inference.ods_discovery.v1";

/// Host API ports commonly used by ODS and native llama (README defaults).
/// Linux Docker host often maps llama-server to **11434**; native paths use **8080**.
pub const ODS_DEFAULT_HOST_PORTS: &[u16] = &[11434, 8080];
