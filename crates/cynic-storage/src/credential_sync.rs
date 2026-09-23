//! Pure credential sync decision — whether SurrealDB runtime credentials
//! are synchronized across service config, password file, and live auth.
//!
//! Known bug this resolves: `/etc/cynic/secrets/storage-runtime-password`
//! contained a hash (`06e57f...`) instead of the plaintext password (`cynic`).
//! `/health` returns 200 WITHOUT auth (false positive); `/rpc` fails with
//! `-u user:pass` when the password is wrong — root cause of
//! `digestion_starvation` (kernel storage_healthy: null despite TCP healthy).

use serde::{Deserialize, Serialize};

/// Observed state of the storage credential surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCredentialObservation {
    /// TCP port is reachable.
    pub tcp_reachable: bool,
    /// HTTP /health returns 200 (no auth required).
    pub health_endpoint_ok: bool,
    /// /rpc accepts credentials (auth actually works).
    pub rpc_auth_success: bool,
    /// Password file exists and is readable.
    pub password_file_present: bool,
    /// Password file content matches the declared password.
    pub password_file_matches: bool,
    /// SurrealDB service is running with the declared password.
    pub service_uses_declared_password: bool,
}

/// Declared credential spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageCredentialSpec {
    pub username: String,
    pub password: String,
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
}

/// Credential sync verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialSyncVerdict {
    /// All surfaces synchronized — runtime can authenticate.
    Synchronized,
    /// One or more surfaces out of sync — repair required.
    OutOfSync {
        reason: String,
        detail: String,
        repair: CredentialRepair,
    },
    /// Cannot determine — residual unknowns.
    Unknown { reason: String, detail: String },
}

/// What repair action is needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialRepair {
    /// Write the correct password to the file.
    SyncPasswordFile,
    /// Restart SurrealDB with the correct password.
    RestartService,
    /// Both file and service need repair.
    FullRepair,
    /// No repair needed (synchronized).
    None,
}

/// Pure decision: are storage credentials synchronized?
pub fn decide_credential_sync(
    spec: &StorageCredentialSpec,
    obs: &StorageCredentialObservation,
) -> CredentialSyncVerdict {
    // Cannot observe anything — unknown.
    if !obs.tcp_reachable {
        return CredentialSyncVerdict::Unknown {
            reason: "storage_tcp_unreachable".into(),
            detail: format!("cannot reach storage endpoint {}", spec.endpoint),
        };
    }

    // TCP up but /rpc auth fails — credentials are out of sync.
    if !obs.rpc_auth_success {
        // Determine repair scope.
        let file_needs_repair = !obs.password_file_present || !obs.password_file_matches;
        let service_needs_repair = !obs.service_uses_declared_password;

        let repair = match (file_needs_repair, service_needs_repair) {
            (true, true) => CredentialRepair::FullRepair,
            (true, false) => CredentialRepair::SyncPasswordFile,
            (false, true) => CredentialRepair::RestartService,
            (false, false) => {
                // File matches, service uses declared password, but auth still fails.
                // Could be a live credential rotation or namespace/database mismatch.
                return CredentialSyncVerdict::Unknown {
                    reason: "storage_auth_failed_despite_matching_credentials".into(),
                    detail: "password file and service config match but /rpc auth fails — check namespace/database".into(),
                };
            }
        };

        return CredentialSyncVerdict::OutOfSync {
            reason: "storage_credentials_out_of_sync".into(),
            detail: format!(
                "rpc auth failed for user '{}' at {} (file_ok={}, service_ok={})",
                spec.username, spec.endpoint, !file_needs_repair, !service_needs_repair
            ),
            repair,
        };
    }

    // /rpc auth succeeds — but check for latent drift (file doesn't match).
    if obs.password_file_present && !obs.password_file_matches {
        return CredentialSyncVerdict::OutOfSync {
            reason: "storage_password_file_drift".into(),
            detail: "rpc auth succeeds but password file content does not match declared password — future restart will fail".into(),
            repair: CredentialRepair::SyncPasswordFile,
        };
    }

    // All surfaces green.
    CredentialSyncVerdict::Synchronized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> StorageCredentialSpec {
        StorageCredentialSpec {
            username: "root".into(),
            password: "cynic".into(),
            endpoint: "http://10.10.10.103:8000".into(),
            namespace: "cynic".into(),
            database: "main".into(),
        }
    }

    fn observation(
        tcp: bool,
        health: bool,
        rpc: bool,
        file_present: bool,
        file_matches: bool,
        service_ok: bool,
    ) -> StorageCredentialObservation {
        StorageCredentialObservation {
            tcp_reachable: tcp,
            health_endpoint_ok: health,
            rpc_auth_success: rpc,
            password_file_present: file_present,
            password_file_matches: file_matches,
            service_uses_declared_password: service_ok,
        }
    }

    #[test]
    fn synchronized_when_all_green() {
        let v = decide_credential_sync(&spec(), &observation(true, true, true, true, true, true));
        assert_eq!(v, CredentialSyncVerdict::Synchronized);
    }

    #[test]
    fn unknown_when_tcp_down() {
        let v = decide_credential_sync(
            &spec(),
            &observation(false, false, false, false, false, false),
        );
        assert!(
            matches!(&v, CredentialSyncVerdict::Unknown { reason, .. } if reason == "storage_tcp_unreachable")
        );
    }

    #[test]
    fn out_of_sync_rpc_fails_file_wrong_service_wrong() {
        let v = decide_credential_sync(
            &spec(),
            &observation(true, true, false, false, false, false),
        );
        assert!(matches!(
            &v,
            CredentialSyncVerdict::OutOfSync {
                repair: CredentialRepair::FullRepair,
                ..
            }
        ));
    }

    #[test]
    fn out_of_sync_rpc_fails_file_ok_service_wrong() {
        let v = decide_credential_sync(&spec(), &observation(true, true, false, true, true, false));
        assert!(matches!(
            &v,
            CredentialSyncVerdict::OutOfSync {
                repair: CredentialRepair::RestartService,
                ..
            }
        ));
    }

    #[test]
    fn out_of_sync_rpc_fails_file_wrong_service_ok() {
        let v = decide_credential_sync(&spec(), &observation(true, true, false, true, false, true));
        assert!(matches!(
            &v,
            CredentialSyncVerdict::OutOfSync {
                repair: CredentialRepair::SyncPasswordFile,
                ..
            }
        ));
    }

    #[test]
    fn password_file_drift_detected() {
        // rpc succeeds but file doesn't match — latent bug.
        let v = decide_credential_sync(&spec(), &observation(true, true, true, true, false, true));
        assert!(matches!(&v,
            CredentialSyncVerdict::OutOfSync { reason, .. } if reason == "storage_password_file_drift"
        ));
    }

    #[test]
    fn unknown_when_auth_fails_despite_matching() {
        let v = decide_credential_sync(&spec(), &observation(true, true, false, true, true, true));
        assert!(matches!(&v,
            CredentialSyncVerdict::Unknown { reason, .. } if reason == "storage_auth_failed_despite_matching_credentials"
        ));
    }
}
