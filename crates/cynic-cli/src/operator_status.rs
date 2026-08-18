//! Read-only operator status projection.
//!
//! This surface deliberately keeps declared inference routing, declared model
//! inventory, and observed local supervisor state separate.  A configured
//! backend or a GGUF on disk is not proof that a model is currently running.

use std::process::ExitCode;

use cynic_inference::{
    LocalModelInventory, LocalRuntimeServiceAny, LocalRuntimeState, LocalRuntimeStatus,
    load_inference_config,
};
use serde::Serialize;

use crate::emit_json;

#[derive(Serialize)]
pub(crate) struct OperatorStatusReport {
    schema: &'static str,
    status: &'static str,
    inference: InferenceStatus,
    next_command: Option<&'static str>,
}

#[derive(Serialize)]
struct InferenceStatus {
    /// Routing intent from inference.toml; this is not a liveness probe.
    configuration: ConfigurationStatus,
    /// Profiles and weights declared in the native vault; this is not a
    /// statement that any profile is running.
    inventory: Observation<LocalModelInventory>,
    /// Read-back from the selected local supervisor.
    local_runtime: Observation<LocalRuntimeStatus>,
    /// True only when the supervisor observation reports an active process.
    actively_supervised: Option<bool>,
    proof_scope: &'static str,
}

#[derive(Serialize)]
struct ConfigurationStatus {
    status: &'static str,
    config_path: Option<String>,
    backend_count: usize,
    sovereign_backend_count: usize,
    error: Option<String>,
}

#[derive(Serialize)]
struct Observation<T> {
    status: &'static str,
    value: Option<T>,
    error: Option<String>,
}

pub(crate) fn execute_operator_status(json: bool) -> ExitCode {
    let report = build_operator_status();

    if json {
        return emit_json(&report, false);
    }

    println!("CYNIC operator status: {}", report.status);
    match report.inference.actively_supervised {
        Some(true) => {
            let runtime = report.inference.local_runtime.value.as_ref();
            println!("  Local LLM supervision: active");
            if let Some(alias) = runtime.and_then(|status| status.alias.as_deref()) {
                println!("  Active model alias: {alias}");
            }
            if let Some(endpoint) = runtime.and_then(|status| status.endpoint.as_deref()) {
                println!("  Endpoint: {endpoint}");
            }
        }
        Some(false) => println!("  Local LLM supervision: inactive"),
        None => println!("  Local LLM supervision: unknown (supervisor observation failed)"),
    }
    println!(
        "  Routing configuration: {} ({} backend(s), {} sovereign)",
        report.inference.configuration.status,
        report.inference.configuration.backend_count,
        report.inference.configuration.sovereign_backend_count
    );
    if let Some(command) = report.next_command {
        println!("  Next read-only/action surface: {command}");
    }
    ExitCode::SUCCESS
}

pub(crate) fn build_operator_status() -> OperatorStatusReport {
    let configuration = observe_configuration();
    let runtime = LocalRuntimeServiceAny::from_env();
    let inventory = observe(runtime.list_inventory());
    let local_runtime = observe(runtime.status());
    let actively_supervised = local_runtime
        .value
        .as_ref()
        .map(|status| status.state == LocalRuntimeState::Active);

    let status = match actively_supervised {
        Some(true) => "active",
        Some(false) => "inactive",
        None => "unknown",
    };
    let next_command = match actively_supervised {
        Some(true) => None,
        Some(false) => Some("cynic models list --json"),
        None => Some("cynic runtime inference-status --json"),
    };
    OperatorStatusReport {
        schema: "cynic.operator.status.v1",
        status,
        inference: InferenceStatus {
            configuration,
            inventory,
            local_runtime,
            actively_supervised,
            proof_scope: "actively_supervised is true only when the selected local supervisor read-back reports state=active; routing configuration and vault inventory remain declared state.",
        },
        next_command,
    }
}

impl OperatorStatusReport {
    pub(crate) fn status(&self) -> &'static str {
        self.status
    }

    pub(crate) fn next_command(&self) -> Option<&'static str> {
        self.next_command
    }
}

fn observe_configuration() -> ConfigurationStatus {
    match load_inference_config() {
        Ok(loaded) => ConfigurationStatus {
            status: "ok",
            config_path: loaded.path.map(|path| path.to_string_lossy().to_string()),
            backend_count: loaded.backends.len(),
            sovereign_backend_count: loaded
                .backends
                .iter()
                .filter(|backend| backend.sovereign)
                .count(),
            error: None,
        },
        Err(cynic_inference::InferenceConfigError::NotFound { path }) => ConfigurationStatus {
            status: "not_found",
            config_path: Some(path.to_string_lossy().to_string()),
            backend_count: 0,
            sovereign_backend_count: 0,
            error: None,
        },
        Err(error) => ConfigurationStatus {
            status: "error",
            config_path: None,
            backend_count: 0,
            sovereign_backend_count: 0,
            error: Some(error.to_string()),
        },
    }
}

fn observe<T, E: ToString>(result: Result<T, E>) -> Observation<T> {
    match result {
        Ok(value) => Observation {
            status: "observed",
            value: Some(value),
            error: None,
        },
        Err(error) => Observation {
            status: "unknown",
            value: None,
            error: Some(error.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::observe;

    #[test]
    fn failed_observation_remains_unknown() {
        let observation = observe::<(), _>(Err("no supervisor"));
        assert_eq!(observation.status, "unknown");
        assert!(observation.value.is_none());
        assert_eq!(observation.error.as_deref(), Some("no supervisor"));
    }
}
