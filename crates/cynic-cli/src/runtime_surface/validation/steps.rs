use std::env;
use std::process::Command as ProcessCommand;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct RuntimeValidationStep {
    pub(super) name: &'static str,
    pub(super) command: Vec<String>,
    pub(super) exit_code: Option<i32>,
    pub(super) expectation: &'static str,
    pub(super) status: &'static str,
    pub(super) observed_schema: Option<String>,
    pub(super) observed_status: Option<String>,
    pub(super) detail: String,
}

pub(super) fn run_json_validation_step_with_predicate(
    name: &'static str,
    args: &[&str],
    allowed_exit_codes: &[i32],
    expectation: &'static str,
    predicate: fn(&Value) -> Result<(), String>,
) -> RuntimeValidationStep {
    let mut step = run_json_validation_step(name, args, allowed_exit_codes, expectation);
    if step.status != "ok" {
        return step;
    }
    let exe = match env::current_exe() {
        Ok(value) => value,
        Err(error) => {
            step.status = "blocked";
            step.detail = format!("failed to resolve current executable for predicate: {error}");
            return step;
        }
    };
    let output = ProcessCommand::new(exe)
        .args(args)
        .env("CYNIC_HOST_PROBE_TIMEOUT_MS", "200")
        .output();
    let Ok(output) = output else {
        step.status = "blocked";
        step.detail = "failed to execute predicate command".to_string();
        return step;
    };
    let parsed = match serde_json::from_slice::<Value>(&output.stdout) {
        Ok(value) => value,
        Err(error) => {
            step.status = "blocked";
            step.detail = format!("predicate JSON parse failed: {error}");
            return step;
        }
    };
    match predicate(&parsed) {
        Ok(()) => {
            step.detail = format!("{}; predicate_ok=true", step.detail);
            step
        }
        Err(error) => {
            step.status = "blocked";
            step.detail = format!("predicate failed: {error}");
            step
        }
    }
}

pub(super) fn run_json_validation_step(
    name: &'static str,
    args: &[&str],
    allowed_exit_codes: &[i32],
    expectation: &'static str,
) -> RuntimeValidationStep {
    let command = command_vec(args);
    let exe = match env::current_exe() {
        Ok(value) => value,
        Err(error) => {
            return RuntimeValidationStep {
                name,
                command,
                exit_code: None,
                expectation,
                status: "blocked",
                observed_schema: None,
                observed_status: None,
                detail: format!("failed to resolve current executable: {error}"),
            };
        }
    };
    let output = ProcessCommand::new(exe)
        .args(args)
        .env("CYNIC_HOST_PROBE_TIMEOUT_MS", "200")
        .output();
    let output = match output {
        Ok(value) => value,
        Err(error) => {
            return RuntimeValidationStep {
                name,
                command,
                exit_code: None,
                expectation,
                status: "blocked",
                observed_schema: None,
                observed_status: None,
                detail: format!("failed to execute command: {error}"),
            };
        }
    };
    let exit_code = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let parsed = serde_json::from_str::<Value>(&stdout);
    let observed_schema = parsed
        .as_ref()
        .ok()
        .and_then(|value| value.get("schema"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let observed_status = parsed
        .as_ref()
        .ok()
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let exit_ok = exit_code.is_some_and(|code| allowed_exit_codes.contains(&code));
    let json_ok = parsed.is_ok();
    let status = if exit_ok && json_ok { "ok" } else { "blocked" };
    let detail = if status == "ok" {
        format!(
            "executed; observed_status={}",
            observed_status.as_deref().unwrap_or("not_reported")
        )
    } else {
        format!(
            "exit_ok={exit_ok} json_ok={json_ok} stderr={} stdout_prefix={}",
            stderr.trim(),
            stdout.chars().take(160).collect::<String>()
        )
    };

    RuntimeValidationStep {
        name,
        command,
        exit_code,
        expectation,
        status,
        observed_schema,
        observed_status,
        detail,
    }
}

fn command_vec(args: &[&str]) -> Vec<String> {
    std::iter::once("cynic".to_string())
        .chain(args.iter().map(|arg| (*arg).to_string()))
        .collect()
}
