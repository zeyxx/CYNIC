//! Local build discovery — binds `BuildDiscoveryObservationPort` to the
//! host's own `cargo metadata` + `rustc` probes. No SSH. Uses
//! `capture_command` and reuses pure `parse_cargo_metadata`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::build_discovery::port::BuildDiscoveryObservationPort;
use crate::domain::build_discovery::probe::parse_cargo_metadata;
use crate::process_observation::capture_command;
use cynic_protocol::build_discovery::BuildDiscoveryReport;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalBuildDiscoveryAdapter;

impl BuildDiscoveryObservationPort for LocalBuildDiscoveryAdapter {
    fn observe(&self) -> Result<BuildDiscoveryReport, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let meta = capture_command("cargo", &["metadata", "--format-version", "1", "--no-deps"])
            .map_err(|e| format!("cargo_metadata_spawn: {e}"))?;
        if !meta.success {
            return Err(format!(
                "cargo_metadata_failed: exit={:?} stderr={}",
                meta.exit_code, meta.stderr
            ));
        }

        let toolchain = capture_command("rustc", &["--version", "--verbose"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout)
            .unwrap_or_else(|| "unknown".to_string());

        let host_triple = capture_command("rustc", &["--print", "host-tuple"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        parse_cargo_metadata(&meta.stdout, now, toolchain, host_triple)
    }
}
