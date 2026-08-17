//! Read-only local Docker observation adapter.

use super::{
    DockerObservationPort, DockerStorageEvidence, DockerStorageReport, normalize_docker_evidence,
};
use crate::process_observation::capture_command;

#[derive(Debug, Clone, Default)]
pub struct LocalDockerAdapter;

impl DockerObservationPort for LocalDockerAdapter {
    fn observe(&self) -> Result<DockerStorageReport, String> {
        let mut evidence = DockerStorageEvidence::default();
        for (label, args) in [
            ("info", vec!["info", "--format", "{{json .}}"]),
            ("system df", vec!["system", "df"]),
            (
                "containers",
                vec!["ps", "--all", "--no-trunc", "--format", "{{json .}}"],
            ),
            (
                "images",
                vec!["image", "ls", "--no-trunc", "--format", "{{json .}}"],
            ),
            ("volumes", vec!["volume", "ls", "--format", "{{json .}}"]),
        ] {
            match capture_command("docker", &args) {
                Ok(output) if output.success => match label {
                    "info" => evidence.info_json = Some(output.stdout),
                    "system df" => evidence.system_df = Some(output.stdout),
                    "containers" => evidence.containers_jsonl = Some(output.stdout),
                    "images" => evidence.images_jsonl = Some(output.stdout),
                    "volumes" => evidence.volumes_jsonl = Some(output.stdout),
                    _ => unreachable!("Docker observation catalog is exhaustive"),
                },
                Ok(output) => evidence.failures.push(format!(
                    "docker {label} failed (exit_code={:?}): {}",
                    output.exit_code,
                    if output.stderr.is_empty() {
                        "no stderr"
                    } else {
                        &output.stderr
                    }
                )),
                Err(error) => evidence
                    .failures
                    .push(format!("docker {label} unavailable: {error}")),
            }
        }
        Ok(normalize_docker_evidence(
            &evidence,
            chrono::Utc::now().timestamp_millis().max(0) as u128,
        ))
    }
}
