#![allow(clippy::print_stdout, clippy::print_stderr)]
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use cynic_kernel::domain::lifecycle_activation_evidence::{
    activate_lifecycle_registry_node, apply_lifecycle_activation_evidence_patch,
    build_lifecycle_service_evidence_patch,
};
use cynic_kernel::domain::lifecycle_activation_requests::{
    activation_request_identity, activation_request_summary,
    pending_activation_task_by_key, validate_activation_request_payload,
};
use cynic_kernel::domain::lifecycle_diagnostics::{
    activation_evidence_gaps, build_activation_evidence_gap_details, lifecycle_blockers,
};
use cynic_kernel::domain::lifecycle_runtime_paths::{
    build_lifecycle_runtime_paths, verify_lifecycle_runtime_paths,
};
use serde_json::{Map, Value};

fn main() -> ExitCode {
    match run() {
        Ok(payload) => {
            println!(
                "{}",
                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<Value, String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        return Err("usage: cynic-kernel-lifecycle-evidence <activation-gap-details|activation-gaps|lifecycle-blockers|runtime-paths|verify-runtime-paths|service-patch|apply-patch|activate-node|validate-activation-request|activation-request-identity|activation-request-summary|pending-activation-index> [...]".to_string());
    };
    match command.as_str() {
        "activation-gap-details" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let env = if options.contains_key("--env-stdin") {
                Some(parse_json_object(&read_stdin()?, "env")?)
            } else {
                None
            };
            build_activation_evidence_gap_details(&registry, env.as_ref())
        }
        "activation-gaps" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let env = if options.contains_key("--env-stdin") {
                Some(parse_json_object(&read_stdin()?, "env")?)
            } else {
                None
            };
            activation_evidence_gaps(&registry, env.as_ref())
        }
        "lifecycle-blockers" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let input = if options.contains_key("--input-stdin") {
                parse_json_object(&read_stdin()?, "input")?
            } else {
                Map::new()
            };
            let fleet_probe = input.get("fleet_probe");
            let fleet_inventory_probe = input.get("fleet_inventory_probe");
            let env = input.get("env").and_then(Value::as_object);
            lifecycle_blockers(&registry, fleet_probe, fleet_inventory_probe, env)
        }
        "runtime-paths" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            build_lifecycle_runtime_paths(&registry)
        }
        "verify-runtime-paths" => {
            let options = parse_options(args)?;
            let node_name = require_option(&options, "--node")?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let home = option_string(&options, "--home");
            let project_slug = option_string(&options, "--project-slug");
            verify_lifecycle_runtime_paths(&node_name, &registry, home, project_slug)
        }
        "service-patch" => {
            let options = parse_options(args)?;
            let node_name = require_option(&options, "--node")?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let service_states = if options.contains_key("--service-states-stdin") {
                let stdin = read_stdin()?;
                let json = parse_json_object(&stdin, "service states")?;
                Some(json)
            } else {
                None
            };
            build_lifecycle_service_evidence_patch(&node_name, &registry, service_states.as_ref())
        }
        "apply-patch" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let stdin = read_stdin()?;
            let patch = parse_json_value(&stdin, "activation evidence patch")?;
            apply_lifecycle_activation_evidence_patch(&patch, &registry)
        }
        "activate-node" => {
            let options = parse_options(args)?;
            let node_name = require_option(&options, "--node")?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            activate_lifecycle_registry_node(&node_name, &registry)
        }
        "validate-activation-request" => {
            let options = parse_options(args)?;
            let registry = load_registry(option_string(&options, "--registry"))?;
            let payload = parse_json_value(&read_stdin()?, "activation request payload")?;
            validate_activation_request_payload(&payload, &registry)
        }
        "activation-request-identity" => {
            let _options = parse_options(args)?;
            let payload = parse_json_value(&read_stdin()?, "activation request payload")?;
            activation_request_identity(&payload)
        }
        "activation-request-summary" => {
            let _options = parse_options(args)?;
            let payload = parse_json_value(&read_stdin()?, "activation request payload")?;
            activation_request_summary(&payload)
        }
        "pending-activation-index" => {
            let _options = parse_options(args)?;
            let payload = parse_json_value(&read_stdin()?, "pending activation tasks")?;
            pending_activation_task_by_key(&payload)
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_options<I>(args: I) -> Result<Map<String, Value>, String>
where
    I: Iterator<Item = String>,
{
    let mut parsed = Map::new();
    let mut pending: Option<String> = None;
    for arg in args {
        if let Some(flag) = pending.take() {
            parsed.insert(flag, Value::String(arg));
            continue;
        }
        match arg.as_str() {
            "--service-states-stdin" | "--env-stdin" | "--input-stdin" => {
                parsed.insert(arg, Value::Bool(true));
            }
            "--node" | "--registry" | "--home" | "--project-slug" => {
                pending = Some(arg);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    if let Some(flag) = pending {
        return Err(format!("missing value for {flag}"));
    }
    Ok(parsed)
}

fn require_option(options: &Map<String, Value>, name: &str) -> Result<String, String> {
    options
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("{name} is required"))
}

fn option_string<'a>(options: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    options.get(name).and_then(Value::as_str)
}

fn load_registry(raw_path: Option<&str>) -> Result<Value, String> {
    let path = raw_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("infra/registry.json"));
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read registry {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse registry {}: {error}", path.display()))
}

fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|error| format!("failed to read stdin: {error}"))?;
    if buf.trim().is_empty() {
        return Err("stdin JSON payload is required".to_string());
    }
    Ok(buf)
}

fn parse_json_value(raw: &str, label: &str) -> Result<Value, String> {
    serde_json::from_str(raw).map_err(|error| format!("invalid {label} JSON: {error}"))
}

fn parse_json_object(raw: &str, label: &str) -> Result<Map<String, Value>, String> {
    parse_json_value(raw, label)?
        .as_object()
        .cloned()
        .ok_or_else(|| format!("{label} must be a JSON object"))
}
