//! CLI-facing factories for the build runner and proof adapters.
//!
//! The CLI driving adapter must not construct runners or proof adapters
//! inline: these factories resolve the runner host from the registry,
//! apply the declared defaults, and wrap the async proof adapter in a
//! blocking call so the surface never spawns a tokio runtime itself.

use cynic_build::build_proof::{BuildProofReport, BuildProofRequest};
use cynic_host::registry::resolve::resolve_build_runner_transport;
use cynic_host::registry::resolve::resolve_source_transport_uri;

use crate::adapters::ssh_build_proof::SshBuildProof;
use crate::ports::build_proof_port::BuildProofPort;
use crate::ports::build_runner_adapter::{
    DEFAULT_CHECKOUT_BASE, DEFAULT_SSOT_URL, LocalCommandRunner, SshBuildRunner,
};

/// Resolve the SSH target for a build trigger.
///
/// An explicit host wins; otherwise the `build_runner` transport is
/// resolved from the registry. Fails closed when the registry cannot
/// answer.
pub fn resolve_runner_host(host: Option<&str>, registry: &str) -> Result<String, String> {
    match host.filter(|value| !value.trim().is_empty()) {
        Some(explicit) => Ok(explicit.to_string()),
        None => resolve_build_runner_transport(Some(registry))
            .map_err(|error| format!("build runner unresolvable in registry {registry}: {error}"))
            .map(|transport| transport.ssh_target),
    }
}

/// Resolve the source-of-truth clone URI for a build trigger.
///
/// An explicit `--ssot-url` wins; otherwise the URI is resolved from the
/// registry's `source_workspace_layout.repositories` for the sovereign
/// `cynic` repository. Fails closed when the registry cannot answer — a
/// clone must never silently fall back to a hardcoded default the runner
/// cannot resolve.
pub fn resolve_ssot_url(ssot_url: &str, registry: &str) -> Result<String, String> {
    match ssot_url.trim() {
        explicit if !explicit.is_empty() => Ok(explicit.to_string()),
        _ => resolve_source_transport_uri(Some(registry), "cynic").map_err(|error| {
            format!("build source unresolvable in registry {registry}: {error}")
        }),
    }
}

/// Construct the `SshBuildRunner` for a CLI build trigger.
///
/// Applies the declared defaults for checkout base and SSOT URL, and
/// normalizes the optional sccache dir. The command runner is always
/// the local argv runner — the SSH hop happens inside the adapter.
pub fn ssh_build_runner(
    host: String,
    user: String,
    checkout_base: String,
    ssot_url: String,
    package: Option<String>,
    sccache_dir: String,
) -> SshBuildRunner {
    SshBuildRunner {
        host,
        user,
        checkout_base: if checkout_base.is_empty() {
            DEFAULT_CHECKOUT_BASE.to_string()
        } else {
            checkout_base
        },
        ssot_url: if ssot_url.is_empty() {
            DEFAULT_SSOT_URL.to_string()
        } else {
            ssot_url
        },
        package,
        sccache_dir: if sccache_dir.trim().is_empty() {
            None
        } else {
            Some(sccache_dir)
        },
        runner: Box::new(LocalCommandRunner),
    }
}

/// Prove a commit builds via the SSH proof adapter, blocking the caller.
///
/// Wraps the async adapter in a tokio runtime so the CLI surface never
/// constructs a runtime itself. The adapter resolves the runner host
/// from the fleet registry when needed.
pub fn prove_build_blocking(
    request: &BuildProofRequest,
    host: &str,
    user: &str,
) -> Result<BuildProofReport, String> {
    // The CLI runs inside a tokio runtime (#[tokio::main]), so a fresh
    // `Runtime::block_on` would panic with "cannot start a runtime from
    // within a runtime". Mirror the adapter convention: join the ambient
    // runtime when present, build one only when absent.
    let adapter = SshBuildProof::new(host, user, "builder");
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(adapter.prove(request))),
        Err(_) => {
            let runtime = tokio::runtime::Runtime::new()
                .map_err(|e| format!("async runtime unavailable: {e}"))?;
            runtime.block_on(adapter.prove(request))
        }
    }
}
