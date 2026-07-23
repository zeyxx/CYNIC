use crate::process_observation::capture_command;

use super::port::StorageCapabilityObservationPort;
use super::types::ProbeObservation;

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalStorageCapabilityObserver;

impl StorageCapabilityObservationPort for LocalStorageCapabilityObserver {
    fn observe_cli_version(&self) -> ProbeObservation {
        command_value("surreal_cli", "surreal", &["version"])
    }

    fn observe_server_version(&self, endpoint: &str) -> ProbeObservation {
        let url = endpoint_url(endpoint, "version");
        command_value(
            "http_version_endpoint",
            "curl",
            &[
                "--fail",
                "--silent",
                "--show-error",
                "--max-time",
                "2",
                &url,
            ],
        )
    }

    fn observe_server_health(&self, endpoint: &str) -> ProbeObservation {
        let url = endpoint_url(endpoint, "health");
        match capture_command(
            "curl",
            &[
                "--fail",
                "--silent",
                "--show-error",
                "--max-time",
                "2",
                &url,
            ],
        ) {
            Ok(output) if output.success => {
                ProbeObservation::observed("http_health_endpoint", "healthy")
            }
            Ok(output) => ProbeObservation::unavailable(
                "http_health_endpoint",
                concise_error(output.exit_code, &output.stderr),
            ),
            Err(error) => ProbeObservation::unavailable("http_health_endpoint", error),
        }
    }

    fn observe_engine(&self) -> ProbeObservation {
        if let Ok(value) = std::env::var("CYNIC_STORAGE_ENGINE")
            && !value.trim().is_empty()
        {
            return ProbeObservation::declared("CYNIC_STORAGE_ENGINE", normalize_engine(&value));
        }

        if let Some(engine) = command_engine(
            "systemd_execstart",
            "systemctl",
            &[
                "show",
                "surrealdb.service",
                "--property=ExecStart",
                "--value",
            ],
        ) {
            return engine;
        }

        if let Some(engine) = command_engine("process_argv", "ps", &["-eo", "args="]) {
            return engine;
        }

        ProbeObservation::unavailable(
            "runtime_process_discovery",
            "storage engine was not declared and no supported engine URI was observed",
        )
    }
}

fn command_value(source: &str, command: &str, args: &[&str]) -> ProbeObservation {
    match capture_command(command, args) {
        Ok(output) if output.success && !output.stdout.trim().is_empty() => {
            ProbeObservation::observed(source, output.stdout.trim())
        }
        Ok(output) => {
            ProbeObservation::unavailable(source, concise_error(output.exit_code, &output.stderr))
        }
        Err(error) => ProbeObservation::unavailable(source, error),
    }
}

fn command_engine(source: &str, command: &str, args: &[&str]) -> Option<ProbeObservation> {
    let output = capture_command(command, args).ok()?;
    if !output.success {
        return None;
    }
    detect_engine(&output.stdout).map(|engine| ProbeObservation::observed(source, engine))
}

fn detect_engine(raw: &str) -> Option<&'static str> {
    let lower = raw.to_ascii_lowercase();
    [
        ("surrealkv://", "surrealkv"),
        ("rocksdb://", "rocksdb"),
        ("tikv://", "tikv"),
        ("surrealds://", "surrealds"),
        ("memory://", "memory"),
        ("mem://", "memory"),
    ]
    .into_iter()
    .find_map(|(marker, engine)| lower.contains(marker).then_some(engine))
}

fn normalize_engine(raw: &str) -> String {
    detect_engine(raw)
        .map(str::to_string)
        .unwrap_or_else(|| raw.trim().to_ascii_lowercase())
}

fn endpoint_url(endpoint: &str, path: &str) -> String {
    format!("{}/{}", endpoint.trim_end_matches('/'), path)
}

fn concise_error(exit_code: Option<i32>, stderr: &str) -> String {
    let detail = stderr.trim();
    if detail.is_empty() {
        format!("command exited with code {exit_code:?}")
    } else {
        format!("exit_code={exit_code:?}: {detail}")
    }
}
