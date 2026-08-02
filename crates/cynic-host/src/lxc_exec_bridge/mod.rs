//! Rust-native, bounded Proxmox LXC bootstrap/recovery bridge.
//!
//! Provider access is an adapter detail. Policy admits exact argv without a
//! shell, execution is confined to `pct exec`, and installation/status remain
//! separate observable surfaces.

mod bundle;
mod decision;
mod execution;
mod types;

pub use bundle::{bundle, install_plan, status};
pub use decision::admit;
pub use execution::execute;
pub use types::{
    LxcBridgeAuthority, LxcBridgeBundleReport, LxcBridgeDecision, LxcBridgeInstallPlan,
    LxcBridgeRequest, LxcBridgeResponse, LxcBridgeStatus,
};
