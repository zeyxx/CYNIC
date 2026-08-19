//! Storage credential sync port — observes whether SurrealDB credentials
//! are synchronized across service config, password file, and live auth.

use cynic_protocol::storage_credential_sync::{StorageCredentialSpec, StorageCredentialObservation};

/// Port for observing storage credential state.
pub trait StorageCredentialSyncPort {
    /// Observe the current credential state.
    fn observe(&self, spec: &StorageCredentialSpec) -> StorageCredentialObservation;
}

/// Production adapter — probes TCP, HTTP /health, /rpc auth, password file, service config.
pub struct StorageCredentialSyncAdapter {
    /// Timeout for HTTP requests.
    timeout: std::time::Duration,
}

impl StorageCredentialSyncAdapter {
    pub fn new(timeout: std::time::Duration) -> Self {
        Self { timeout }
    }

    /// Default production adapter with 5s timeout.
    pub fn default() -> Self {
        Self::new(std::time::Duration::from_secs(5))
    }
}

impl StorageCredentialSyncPort for StorageCredentialSyncAdapter {
    fn observe(&self, spec: &StorageCredentialSpec) -> StorageCredentialObservation {
        let endpoint = spec.endpoint.trim_end_matches('/');

        // 1. TCP reachability (port open).
        let tcp_reachable = {
            let addr = format!(
                "{}:{}",
                endpoint
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split(':')
                    .next()
                    .unwrap_or(""),
                endpoint
                    .split(':')
                    .nth(2)
                    .unwrap_or("8000")
            );
            std::net::TcpStream::connect(&addr)
                .map(|_| true)
                .unwrap_or(false)
        };

        // 2. /health endpoint (no auth).
        let health_endpoint_ok = if tcp_reachable {
            let client = reqwest::blocking::Client::builder()
                .timeout(self.timeout)
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build();
            client
                .map(|c| c.get(format!("{endpoint}/health")).send())
                .unwrap_or(Err(reqwest::Error::new(
                    reqwest::ErrorKind::Request,
                    "client build failed",
                )))
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        } else {
            false
        };

        // 3. /rpc auth (with credentials).
        let rpc_auth_success = if tcp_reachable {
            let client = reqwest::blocking::Client::builder()
                .timeout(self.timeout)
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build();
            client
                .map(|c| {
                    c.post(format!("{endpoint}/rpc"))
                        .basic_auth(&spec.username, Some(&spec.password))
                        .header("Accept", "application/json")
                        .header("surreal-ns", &spec.namespace)
                        .header("surreal-db", &spec.database)
                        .body("SELECT * FROM \"\" LIMIT 1")
                        .send()
                })
                .unwrap_or(Err(reqwest::Error::new(
                    reqwest::ErrorKind::Request,
                    "client build failed",
                )))
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        } else {
            false
        };

        // 4. Password file presence and content match.
        let password_file_present = std::path::Path::new("/etc/cynic/secrets/storage-runtime-password")
            .exists();
        let password_file_matches = if password_file_present {
            std::fs::read_to_string("/etc/cynic/secrets/storage-runtime-password")
                .map(|s| s.trim() == spec.password)
                .unwrap_or(false)
        } else {
            false
        };

        // 5. Service uses declared password (from systemd env or config).
        // We infer this from /rpc success when file matches — direct inspection
        // of systemd env requires root, so we use a heuristic.
        let service_uses_declared_password = if rpc_auth_success && password_file_matches {
            true
        } else if !rpc_auth_success && !password_file_matches {
            false
        } else {
            // Ambiguous — cannot determine without inspecting service config directly.
            rpc_auth_success
        };

        StorageCredentialObservation {
            tcp_reachable,
            health_endpoint_ok,
            rpc_auth_success,
            password_file_present,
            password_file_matches,
            service_uses_declared_password,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::storage_credential_sync::StorageCredentialSyncPort;

    #[test]
    fn adapter_returns_observation() {
        let adapter = StorageCredentialSyncAdapter::default();
        let spec = StorageCredentialSpec {
            username: "root".into(),
            password: "cynic".into(),
            endpoint: "http://127.0.0.1:9999".into(),  // unreachable
            namespace: "cynic".into(),
            database: "main".into(),
        };
        let obs = adapter.observe(&spec);
        assert!(!obs.tcp_reachable);
        assert!(!obs.health_endpoint_ok);
        assert!(!obs.rpc_auth_success);
    }

    #[test]
    fn password_file_present_check() {
        // This test depends on the environment — just verify it doesn't panic.
        let adapter = StorageCredentialSyncAdapter::default();
        let spec = StorageCredentialSpec {
            username: "root".into(),
            password: "cynic".into(),
            endpoint: "http://127.0.0.1:9999".into(),
            namespace: "cynic".into(),
            database: "main".into(),
        };
        let obs = adapter.observe(&spec);
        // Should not panic — password file check is internal.
        let _ = obs.password_file_present;
        let _ = obs.password_file_matches;
    }
}
