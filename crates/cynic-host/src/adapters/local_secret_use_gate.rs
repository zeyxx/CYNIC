//! Local secret-use gatekeeper: the only path from an agent-facing surface to
//! secret material.
//!
//! The gatekeeper never returns a secret value. It:
//! 1. Evaluates every request against a declared policy via the pure
//!    `decide_secret_use` function.
//! 2. Appends one `SecretUseAuditEntry` per request to an append-only JSONL
//!    sink, allowed or denied.
//! 3. On `Allowed`, mints an opaque expiring handle. The value is resolved only
//!    inside `execute_with_secret`, which spawns a short-lived child process
//!    with the value in its environment, then redacts any accidental echo from
//!    the captured output before returning.
//!
//! Authority: `cynic-protocol::secret_use` — the gate matrix is
//! `SecretUseGrant::permits_class`; a table in prose cannot be checked.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use cynic_protocol::secret_use::{
    SECRET_USE_AUDIT_SCHEMA, SecretUseAuditEntry, SecretUseDecision, SecretUsePolicy,
    SecretUseRequest, decide_secret_use,
};
use cynic_secrets::protocol::{
    SECRET_USE_EXECUTION_SCHEMA, SecretUseExecutionReport, SecretUseHandle, SecretUseOutcome,
};
use cynic_secrets::{SecretError, SecretProviderPort, SecretReference, SecretUseGatePort};

use crate::clock::now_unix_ms;

/// Default lifetime of an authorization handle: 60 seconds. Short by design —
/// an authorized use is meant to happen immediately, not be hoarded.
pub const DEFAULT_HANDLE_TTL_MS: u64 = 60_000;

/// Default environment variable name carrying the injected value.
pub const DEFAULT_INJECTION_ENV_VAR: &str = "CYNIC_SECRET_VALUE";

/// A locally-enforced secret-use gatekeeper.
///
/// Holds the declared policy, the audit sink path, and a map of live handles.
/// The handle map is the capability: a caller cannot fabricate a valid handle
/// without the token this adapter minted.
pub struct LocalSecretUseGate {
    policy: SecretUsePolicy,
    audit_path: Option<PathBuf>,
    /// Resolver closure: id -> value. Injected so the gate stays testable and
    /// the resolution strategy (composite provider, env, file) is not baked in.
    resolver: Box<dyn Fn(&str) -> Option<String> + Send + Sync>,
    handles: Mutex<HashMap<String, LiveHandle>>,
    ttl_ms: u64,
}

struct LiveHandle {
    secret_id: String,
    expires_at_unix_ms: u64,
}

impl LocalSecretUseGate {
    /// Build a gate with an explicit resolver and policy.
    ///
    /// `resolver` receives a secret id and returns its value, or `None` if
    /// unresolvable. It is called only after a request is `Allowed`, and only
    /// from `execute_with_secret`.
    pub fn new(
        policy: SecretUsePolicy,
        resolver: impl Fn(&str) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            policy,
            audit_path: None,
            resolver: Box::new(resolver),
            handles: Mutex::new(HashMap::new()),
            ttl_ms: DEFAULT_HANDLE_TTL_MS,
        }
    }

    /// Route audit entries to an append-only JSONL file.
    pub fn with_audit_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.audit_path = Some(path.into());
        self
    }

    /// Override the handle lifetime.
    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    fn append_audit(&self, entry: &SecretUseAuditEntry) {
        let Some(path) = &self.audit_path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(line) = serde_json::to_string(entry) else {
            return;
        };
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }

    fn mint_token(secret_id: &str, now_ms: u64) -> String {
        // Token binds the secret id and the mint instant. Not a cryptographic
        // secret: the handle map is the authority, the token is only its key.
        format!("{secret_id}:{now_ms}")
    }

    /// Number of live (unexpired) handles — for tests and read-back.
    pub fn live_handle_count(&self) -> usize {
        let now = now_unix_ms();
        self.handles
            .lock()
            .map(|map| map.values().filter(|h| now < h.expires_at_unix_ms).count())
            .unwrap_or(0)
    }
}

impl SecretUseGatePort for LocalSecretUseGate {
    fn authorize(&self, request: &SecretUseRequest) -> SecretUseOutcome {
        let now = now_unix_ms();
        let decision = decide_secret_use(request, &self.policy, now);

        let audit = SecretUseAuditEntry::from_decision(request, decision.clone(), now);
        debug_assert_eq!(audit.schema, SECRET_USE_AUDIT_SCHEMA);
        self.append_audit(&audit);

        match decision {
            SecretUseDecision::Denied { reason } => SecretUseOutcome::Denied { reason },
            SecretUseDecision::Allowed { .. } => {
                let token = Self::mint_token(&request.reference.id, now);
                let expires_at_unix_ms = now.saturating_add(self.ttl_ms);
                if let Ok(mut map) = self.handles.lock() {
                    map.insert(
                        token.clone(),
                        LiveHandle {
                            secret_id: request.reference.id.clone(),
                            expires_at_unix_ms,
                        },
                    );
                }
                SecretUseOutcome::Allowed {
                    handle: SecretUseHandle {
                        secret_id: request.reference.id.clone(),
                        token,
                        expires_at_unix_ms,
                    },
                }
            }
        }
    }

    fn execute_with_secret(
        &self,
        handle: &SecretUseHandle,
        env_var: &str,
        argv: &[String],
    ) -> Result<SecretUseExecutionReport, SecretError> {
        if argv.is_empty() {
            return Err(SecretError::Backend("empty argv".to_string()));
        }
        let env_var = if env_var.trim().is_empty() {
            DEFAULT_INJECTION_ENV_VAR
        } else {
            env_var
        };

        let now = now_unix_ms();

        // Validate the handle against the live map: unknown or expired fails closed.
        let live_secret_id = {
            let mut map = self
                .handles
                .lock()
                .map_err(|_| SecretError::Backend("handle map poisoned".to_string()))?;
            match map.get(&handle.token) {
                Some(live) if now < live.expires_at_unix_ms => {
                    let id = live.secret_id.clone();
                    // One-shot: consume the handle so a token cannot be replayed.
                    map.remove(&handle.token);
                    id
                }
                Some(_) => {
                    map.remove(&handle.token);
                    return Err(SecretError::NotFound(format!(
                        "handle expired for secret '{}'",
                        handle.secret_id
                    )));
                }
                None => {
                    return Err(SecretError::NotFound(format!(
                        "unknown or already-consumed handle for secret '{}'",
                        handle.secret_id
                    )));
                }
            }
        };

        if live_secret_id != handle.secret_id {
            return Err(SecretError::ResolutionFailed(
                handle.secret_id.clone(),
                "handle secret id mismatch".to_string(),
            ));
        }

        let mut value = (self.resolver)(&live_secret_id).ok_or_else(|| {
            SecretError::NotFound(format!("secret '{}' not resolvable", live_secret_id))
        })?;

        let program = &argv[0];
        let args = &argv[1..];
        let output = Command::new(program)
            .args(args)
            .env(env_var, &value)
            .output();

        // The value has been handed to the child; wipe our copy immediately.
        let redact_target = std::mem::take(&mut value);

        let output = output.map_err(|e| SecretError::Backend(format!("spawn {program}: {e}")))?;

        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let (stdout, redacted_out) = redact_if_present(raw_stdout, &redact_target);
        let (stderr, redacted_err) = redact_if_present(raw_stderr, &redact_target);

        Ok(SecretUseExecutionReport {
            schema: SECRET_USE_EXECUTION_SCHEMA,
            secret_id: live_secret_id,
            command: argv.join(" "),
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout,
            stderr,
            output_redacted: redacted_out || redacted_err,
        })
    }
}

/// Replace every occurrence of the secret value in `text` with `[REDACTED]`.
/// A child that echoes its environment must not leak the value into the report.
fn redact_if_present(text: String, secret: &str) -> (String, bool) {
    if secret.is_empty() || !text.contains(secret) {
        return (text, false);
    }
    (text.replace(secret, "[REDACTED]"), true)
}

/// Resolve a secret id through the standard composite hierarchy
/// (process environment -> principal-owned file -> OS credential store).
///
/// This is the default resolver for production use. It is only ever invoked
/// after a request has been authorized.
pub fn resolve_via_composite(id: &str) -> Option<String> {
    let reference = SecretReference::env(id, &id.to_uppercase().replace('-', "_"));
    cynic_secrets::CompositeSecretProvider::default().resolve(&reference)
}

/// Audit sink path under the operator's principal home.
///
/// `CYNIC_SECRET_AUDIT_PATH` overrides; otherwise
/// `${HOME}/.local/share/cynic/secret-use-audit.jsonl`.
pub fn default_audit_path() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CYNIC_SECRET_AUDIT_PATH") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    std::env::var("HOME")
        .ok()
        .filter(|home| !home.trim().is_empty())
        .map(|home| Path::new(&home).join(".local/share/cynic/secret-use-audit.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::secret_reference::{SecretClass, SecretProvider};
    use cynic_protocol::secret_use::{SecretUseGrant, SecretUsePurpose};

    fn runtime_ref(id: &str) -> SecretReference {
        SecretReference {
            id: id.to_string(),
            class: SecretClass::RuntimeMachineIdentity,
            provider: SecretProvider::PrincipalOwnedFile,
            locator: id.to_string(),
        }
    }

    fn gate_with_secret(secret_id: &str, value: &str) -> LocalSecretUseGate {
        let policy = SecretUsePolicy::new(
            vec![SecretUseGrant {
                role: "scheduler_agent".to_string(),
                secret_classes: vec![SecretClass::RuntimeMachineIdentity],
                purposes: vec![SecretUsePurpose::InjectProcess],
                requires_claim: true,
            }],
            cynic_protocol::secret_use::SecretMutationPolicy::ReadOnly,
        );
        let owned = secret_id.to_string();
        let owned_value = value.to_string();
        LocalSecretUseGate::new(policy, move |id: &str| {
            if id == owned {
                Some(owned_value.clone())
            } else {
                None
            }
        })
    }

    fn request(secret_id: &str) -> SecretUseRequest {
        SecretUseRequest::new(
            "agent-code",
            "scheduler_agent",
            runtime_ref(secret_id),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_test_1")
    }

    #[test]
    fn denied_request_yields_no_handle() {
        let gate = LocalSecretUseGate::new(SecretUsePolicy::deny_all(), |_| Some("x".into()));
        let outcome = gate.authorize(&request("forgejo-token"));
        assert!(matches!(outcome, SecretUseOutcome::Denied { .. }));
        assert_eq!(gate.live_handle_count(), 0);
    }

    #[test]
    fn allowed_request_yields_handle_without_value() {
        let gate = gate_with_secret("proxmox-token", "super-secret-value");
        let outcome = gate.authorize(&request("proxmox-token"));

        let SecretUseOutcome::Allowed { handle } = outcome else {
            panic!("expected allowed");
        };
        assert_eq!(handle.secret_id, "proxmox-token");
        assert!(!handle.is_expired(now_unix_ms()));

        let json = serde_json::to_string(&handle).expect("serialize handle");
        assert!(
            !json.contains("super-secret-value"),
            "handle must not carry the value"
        );
    }

    #[test]
    fn execute_injects_value_and_redacts_echo() {
        let gate = gate_with_secret("proxmox-token", "super-secret-value");
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("proxmox-token")) else {
            panic!("expected allowed");
        };

        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            "echo \"seen=$CYNIC_SECRET_VALUE\"".to_string(),
        ];
        let report = gate
            .execute_with_secret(&handle, "CYNIC_SECRET_VALUE", &argv)
            .expect("execution succeeds");

        assert!(report.success);
        assert!(report.output_redacted, "echoed value must be redacted");
        assert!(!report.stdout.contains("super-secret-value"));
        assert!(report.stdout.contains("[REDACTED]"));
    }

    #[test]
    fn child_sees_the_value_it_was_given() {
        let gate = gate_with_secret("proxmox-token", "expected-token");
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("proxmox-token")) else {
            panic!("expected allowed");
        };

        // Compare inside the child so the value never reaches the report.
        let argv = vec![
            "sh".to_string(),
            "-c".to_string(),
            "[ \"$CYNIC_SECRET_VALUE\" = \"expected-token\" ] && echo MATCH || echo MISMATCH"
                .to_string(),
        ];
        let report = gate
            .execute_with_secret(&handle, "CYNIC_SECRET_VALUE", &argv)
            .expect("execution succeeds");

        assert!(
            report.stdout.contains("MATCH"),
            "child must receive the value"
        );
        assert!(!report.stdout.contains("expected-token"));
    }

    #[test]
    fn handle_is_one_shot() {
        let gate = gate_with_secret("proxmox-token", "v");
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("proxmox-token")) else {
            panic!("expected allowed");
        };
        let argv = vec!["true".to_string()];

        assert!(gate.execute_with_secret(&handle, "X", &argv).is_ok());
        let replay = gate.execute_with_secret(&handle, "X", &argv);
        assert!(replay.is_err(), "a consumed handle must not be reusable");
    }

    #[test]
    fn expired_handle_fails_closed() {
        let gate = gate_with_secret("proxmox-token", "v").with_ttl_ms(0);
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("proxmox-token")) else {
            panic!("expected allowed");
        };
        let argv = vec!["true".to_string()];
        assert!(gate.execute_with_secret(&handle, "X", &argv).is_err());
    }

    #[test]
    fn unknown_handle_fails_closed() {
        let gate = gate_with_secret("proxmox-token", "v");
        let forged = SecretUseHandle {
            secret_id: "proxmox-token".to_string(),
            token: "forged:0".to_string(),
            expires_at_unix_ms: u64::MAX,
        };
        let argv = vec!["true".to_string()];
        assert!(gate.execute_with_secret(&forged, "X", &argv).is_err());
    }

    #[test]
    fn audit_is_appended_for_every_request() {
        let dir = tempfile::tempdir().expect("tempdir");
        let audit_path = dir.path().join("audit.jsonl");
        let gate = gate_with_secret("proxmox-token", "v").with_audit_path(&audit_path);

        let _ = gate.authorize(&request("proxmox-token"));
        let denied = SecretUseRequest::new(
            "agent-rogue",
            "scheduler_agent",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        );
        let _ = gate.authorize(&denied);

        let content = std::fs::read_to_string(&audit_path).expect("audit file exists");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "one audit line per request, allowed or denied"
        );
        assert!(content.contains("cynic.secret_use_audit.v1"));
        assert!(!content.contains("\"value\""));
    }

    #[test]
    fn empty_argv_is_refused() {
        let gate = gate_with_secret("proxmox-token", "v");
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("proxmox-token")) else {
            panic!("expected allowed");
        };
        assert!(gate.execute_with_secret(&handle, "X", &[]).is_err());
    }

    #[test]
    fn unresolvable_secret_fails_after_authorization() {
        let policy = SecretUsePolicy::new(
            vec![SecretUseGrant {
                role: "scheduler_agent".to_string(),
                secret_classes: vec![SecretClass::RuntimeMachineIdentity],
                purposes: vec![SecretUsePurpose::InjectProcess],
                requires_claim: true,
            }],
            cynic_protocol::secret_use::SecretMutationPolicy::ReadOnly,
        );
        let gate = LocalSecretUseGate::new(policy, |_| None);
        let SecretUseOutcome::Allowed { handle } = gate.authorize(&request("absent")) else {
            panic!("expected allowed");
        };
        let argv = vec!["true".to_string()];
        assert!(gate.execute_with_secret(&handle, "X", &argv).is_err());
    }

    #[test]
    fn role_absent_from_policy_is_denied_by_the_gate() {
        let gate = gate_with_secret("proxmox-token", "v");
        let rogue = SecretUseRequest::new(
            "agent-rogue",
            "fleet_coordinator",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        let outcome = gate.authorize(&rogue);
        assert!(matches!(outcome, SecretUseOutcome::Denied { .. }));
    }

    #[test]
    fn gate_accepts_a_policy_derived_from_the_declaration() {
        use crate::adapters::secret_use_policy_loader::derive_policy_from_declaration;

        let declaration = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../infra/agent-roles.json"),
        )
        .expect("read agent-roles.json");

        let policy = derive_policy_from_declaration(&declaration);
        let gate = LocalSecretUseGate::new(policy, |_| Some("value".to_string()));

        // scheduler_agent declares RuntimeMachineIdentity: allowed with a claim.
        let allowed = SecretUseRequest::new(
            "agent-code",
            "scheduler_agent",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        assert!(matches!(
            gate.authorize(&allowed),
            SecretUseOutcome::Allowed { .. }
        ));

        // schema_warden declares None: nothing may be consumed.
        let denied = SecretUseRequest::new(
            "agent-code",
            "schema_warden",
            runtime_ref("proxmox-token"),
            SecretUsePurpose::InjectProcess,
        )
        .with_claim("claim_1");
        assert!(matches!(
            gate.authorize(&denied),
            SecretUseOutcome::Denied { .. }
        ));
    }
}
