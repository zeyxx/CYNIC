use cynic_lifecycle::{ArgvOutput, ArgvRunner};
use cynic_protocol::{STORAGE_OBSERVATION_SCHEMA, StorageObservation, StoragePhase};

use crate::ports::storage_guest_port::{StorageGuestObservationPort, StorageGuestTarget};

/// Adapter for a guest command transport already bound by the caller.
/// Commands are argv-only and every result remains observable in the typed
/// storage observation; this adapter is not itself a remote transport.
pub struct ArgvStorageGuestObserver<R> {
    runner: R,
    service_manager_bin: String,
    df_bin: String,
    curl_bin: String,
}

impl<R> ArgvStorageGuestObserver<R> {
    pub fn new(
        runner: R,
        service_manager_bin: impl Into<String>,
        df_bin: impl Into<String>,
        curl_bin: impl Into<String>,
    ) -> Self {
        Self {
            runner,
            service_manager_bin: service_manager_bin.into(),
            df_bin: df_bin.into(),
            curl_bin: curl_bin.into(),
        }
    }
}

impl<R: ArgvRunner> StorageGuestObservationPort for ArgvStorageGuestObserver<R> {
    async fn observe_guest(
        &self,
        target: &StorageGuestTarget,
    ) -> Result<StorageObservation, String> {
        validate_target(target)?;
        let service = self.runner.run(&[
            &self.service_manager_bin,
            "is-active",
            "--quiet",
            &target.service,
        ]);
        let filesystem = self.runner.run(&[
            &self.df_bin,
            "-B1",
            "--output=size,avail",
            &target.mount_path,
        ]);
        let health = self.runner.run(&[
            &self.curl_bin,
            "--fail",
            "--silent",
            "--show-error",
            &target.health_endpoint,
        ]);
        let (filesystem_bytes, free_bytes) = parse_df_output(&filesystem.stdout);
        let service_ready = parse_service_active(&service);
        let health_ok = if health.spawn_error.is_some() {
            None
        } else {
            Some(health.success())
        };
        let mut unknowns = Vec::new();
        if filesystem_bytes.is_none() || free_bytes.is_none() {
            unknowns.push("guest_filesystem_capacity_not_read_back".to_string());
        }
        if service_ready.is_none() {
            unknowns.push("storage_service_state_not_read_back".to_string());
        }
        if health_ok != Some(true) {
            unknowns.push("storage_service_health_not_ready".to_string());
        }
        Ok(StorageObservation {
            schema: STORAGE_OBSERVATION_SCHEMA.to_string(),
            volume_id: target.volume_id.clone(),
            phase: if unknowns.is_empty() {
                StoragePhase::Ready
            } else {
                StoragePhase::Unknown
            },
            provider_bytes: None,
            filesystem_bytes,
            free_bytes,
            service_ready: service_ready
                .zip(health_ok)
                .map(|(active, healthy)| active && healthy),
            evidence_refs: vec![
                "guest_service_state_read_back".to_string(),
                "guest_filesystem_capacity_read_back".to_string(),
                "guest_service_health_read_back".to_string(),
            ],
            unknowns,
        })
    }
}

fn validate_target(target: &StorageGuestTarget) -> Result<(), String> {
    if target.service.is_empty()
        || target.mount_path.is_empty()
        || target.health_endpoint.is_empty()
    {
        return Err("storage guest target fields must be non-empty".to_string());
    }
    if [&target.service, &target.mount_path, &target.health_endpoint]
        .iter()
        .any(|value| value.contains('\0'))
    {
        return Err("storage guest target contains NUL".to_string());
    }
    Ok(())
}

fn parse_service_active(output: &ArgvOutput) -> Option<bool> {
    if output.spawn_error.is_some() {
        return None;
    }
    Some(output.success())
}

fn parse_df_output(stdout: &str) -> (Option<u64>, Option<u64>) {
    stdout
        .lines()
        .rev()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let size = fields.next()?.parse::<u64>().ok()?;
            let available = fields.next()?.parse::<u64>().ok()?;
            Some((Some(size), Some(available)))
        })
        .unwrap_or((None, None))
}

#[cfg(test)]
mod tests {
    use super::{parse_df_output, parse_service_active};
    use cynic_lifecycle::ArgvOutput;

    #[test]
    fn parses_machine_readable_df() {
        assert_eq!(
            parse_df_output("Size Avail\n8589934592 7000000000\n"),
            (Some(8589934592), Some(7000000000))
        );
    }

    #[test]
    fn preserves_unknown_service_spawn() {
        assert_eq!(
            parse_service_active(&ArgvOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: Some("missing".to_string()),
            }),
            None
        );
    }
}
