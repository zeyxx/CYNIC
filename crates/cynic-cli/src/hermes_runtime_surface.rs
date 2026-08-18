use std::process::ExitCode;

use cynic_inference::HermesGatewayService;

use crate::runtime_surface::RuntimeCommand;

pub(crate) fn try_execute(command: RuntimeCommand) -> Result<ExitCode, RuntimeCommand> {
    match command {
        RuntimeCommand::HermesStatus { json } => Ok(status(json)),
        RuntimeCommand::HermesStart { json, no_wait } => Ok(start(json, !no_wait)),
        RuntimeCommand::HermesStop { json } => Ok(stop(json)),
        other => Err(other),
    }
}

fn status(json: bool) -> ExitCode {
    let service = HermesGatewayService::from_process();
    match service.status() {
        Ok(status) => emit(json, &status, status.health_ok && status.capabilities_ok),
        Err(error) => fail(json, error.to_string()),
    }
}

fn start(json: bool, wait_for_ready: bool) -> ExitCode {
    let service = HermesGatewayService::from_process();
    match service.start_with_transition(wait_for_ready, "cli-hermes-start") {
        Ok(report) => emit(json, &report, report.mutation.ok),
        Err(error) => fail(json, error.to_string()),
    }
}

fn stop(json: bool) -> ExitCode {
    let service = HermesGatewayService::from_process();
    match service.stop() {
        Ok(report) => emit(json, &report, report.ok),
        Err(error) => fail(json, error.to_string()),
    }
}

fn emit<T: serde::Serialize>(json: bool, value: &T, ok: bool) -> ExitCode {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|error| {
                format!("{{\"status\":\"failed\",\"error\":\"{error}\"}}")
            })
        );
    } else {
        println!("{}", serde_json::to_string(value).unwrap_or_default());
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn fail(json: bool, error: String) -> ExitCode {
    if json {
        println!(
            "{{\"status\":\"failed\",\"error\":{}}}",
            serde_json::to_string(&error).unwrap_or_else(|_| "\"unknown\"".into())
        );
    } else {
        eprintln!("hermes runtime failed: {error}");
    }
    ExitCode::FAILURE
}
