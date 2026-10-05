//! Node observation commands — local observation and registry observation envelope.
//!
//! Merged from `node_surface/cli.rs` (16 variants) into the unified node surface.

use std::process::ExitCode;

pub(crate) fn execute_observe(
    registry: &str,
    fail_on_warning: bool,
    fail_on_critical: bool,
) -> ExitCode {
    crate::node_surface::network_surfaces_uc::execute_observe(
        registry,
        fail_on_warning,
        fail_on_critical,
    )
}

pub(crate) fn execute_registry_observation(
    registry: &str,
    output: Option<&str>,
) -> ExitCode {
    crate::node_surface::network_surfaces_uc::execute_registry_observation(
        registry,
        output,
    )
}
