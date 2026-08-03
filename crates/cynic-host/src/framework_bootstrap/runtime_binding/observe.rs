use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

use super::types::{RUNTIME_BINDING_PLAN_SCHEMA, RuntimeBindingObservation, RuntimeBindingSpec};

pub fn observe_runtime_binding(spec: &RuntimeBindingSpec) -> RuntimeBindingObservation {
    let mut unknowns = Vec::new();
    let environment = read_optional(
        Path::new(&spec.environment_path),
        &mut unknowns,
        "environment",
    );
    let observed_rest_addr = environment
        .as_deref()
        .and_then(|content| environment_value(content, "CYNIC_REST_ADDR"));
    let unit = read_optional(
        Path::new(&spec.service_unit_path),
        &mut unknowns,
        "service_unit",
    );
    let endpoint_reachable = resolve_socket(&spec.rest_addr(), &mut unknowns)
        .map(|socket| TcpStream::connect_timeout(&socket, Duration::from_secs(2)).is_ok());

    RuntimeBindingObservation {
        schema: RUNTIME_BINDING_PLAN_SCHEMA,
        environment_present: environment.is_some(),
        environment_binding_matches: observed_rest_addr
            .as_ref()
            .map(|value| value == &spec.rest_addr()),
        observed_rest_addr,
        service_unit_present: unit.is_some(),
        service_unit_consumes_environment: unit.as_ref().map(|content| {
            content
                .lines()
                .any(|line| line.trim() == format!("EnvironmentFile=-{}", spec.environment_path))
        }),
        endpoint_reachable,
        unknowns,
        evidence_refs: vec![
            format!("runtime_binding_environment:{}", spec.environment_path),
            format!("runtime_binding_service_unit:{}", spec.service_unit_path),
            format!("runtime_binding_endpoint:{}", spec.endpoint_url()),
        ],
    }
}

pub(super) fn render_environment_projection(existing: Option<&str>, rest_addr: &str) -> String {
    let mut lines = existing
        .into_iter()
        .flat_map(str::lines)
        .filter(|line| !line.trim_start().starts_with("CYNIC_REST_ADDR="))
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.push(format!("CYNIC_REST_ADDR={rest_addr}"));
    format!("{}\n", lines.join("\n"))
}

fn environment_value(content: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    content
        .lines()
        .filter_map(|line| line.trim().strip_prefix(&prefix))
        .last()
        .map(str::to_string)
}

fn read_optional(path: &Path, unknowns: &mut Vec<String>, label: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            unknowns.push(format!("runtime_binding_{label}_unreadable:{error}"));
            None
        }
    }
}

fn resolve_socket(value: &str, unknowns: &mut Vec<String>) -> Option<SocketAddr> {
    match value.to_socket_addrs() {
        Ok(mut addresses) => addresses.next().or_else(|| {
            unknowns.push("runtime_binding_endpoint_resolution_empty".to_string());
            None
        }),
        Err(error) => {
            unknowns.push(format!(
                "runtime_binding_endpoint_resolution_failed:{error}"
            ));
            None
        }
    }
}
