use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;

use crate::domain::storage_lifecycle::StorageObservation;
use crate::ports::storage_lifecycle_port::StorageLifecyclePort;

#[derive(Debug, Clone)]
pub struct LocalStorageLifecycleAdapter {
    client: Client,
}

impl Default for LocalStorageLifecycleAdapter {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl StorageLifecyclePort for LocalStorageLifecycleAdapter {
    async fn observe(&self, service: &str, health_url: &str) -> StorageObservation {
        let observed_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis() as u64);
        let service_output = Command::new("systemctl")
            .args(["is-active", service])
            .output();
        let (service_active, service_state, mut observation_error) = match service_output {
            Ok(output) => (
                output.status.success(),
                String::from_utf8_lossy(&output.stdout).trim().to_string(),
                None,
            ),
            Err(error) => (
                false,
                "unknown".to_string(),
                Some(format!("systemctl_observation_failed:{error}")),
            ),
        };
        let health = self.client.get(health_url).send().await;
        let (health_ok, health_status) = match health {
            Ok(response) => {
                let status = response.status();
                (status.is_success(), Some(status.as_u16()))
            }
            Err(error) => {
                if observation_error.is_none() {
                    observation_error = Some(format!("health_observation_failed:{error}"));
                }
                (false, None)
            }
        };

        StorageObservation {
            observed_at_unix_ms,
            service_active,
            service_state,
            health_ok,
            health_status,
            observation_error,
        }
    }

    fn restart(&self, service: &str) -> Result<(), String> {
        let status = Command::new("systemctl")
            .args(["restart", service])
            .status()
            .map_err(|error| format!("restart_spawn_failed:{error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("restart_failed:{status}"))
        }
    }
}
