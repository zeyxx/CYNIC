use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::Duration;

use cynic_protocol::{
    STORAGE_IDENTITY_OBSERVATION_SCHEMA, STORAGE_IDENTITY_SPEC_SCHEMA, StorageEvidenceState,
    StorageIdentityObservation, StorageIdentityRole, StorageIdentitySpec,
    reconcile_storage_identity,
};

use crate::runtime_secret_install::RuntimeSecretInstallSpec;
use crate::secret_material::SecretMaterial;
use crate::storage_identity::{
    LocalStorageIdentitySecretAdapter, StorageIdentityEnrollmentLease, StorageIdentitySecretPort,
    SurrealStorageIdentityAdminAdapter, enroll_storage_identity,
};

const DEFAULT_RUNTIME_USERNAME: &str = "cynic-runtime";
const DEFAULT_TOKEN_DURATION: &str = "15m";
const DEFAULT_SESSION_DURATION: &str = "12h";
const DEFAULT_TIMEOUT_SECONDS: u64 = 10;

pub(crate) fn execute(args: &[String]) -> Result<ExitCode, String> {
    let parsed = ParsedArgs::parse(args)?;
    let spec = parsed.spec();
    if !parsed.apply {
        return emit_preflight_plan(&spec);
    }
    if unsafe { libc::geteuid() } != 0 {
        return Err("storage identity enrollment requires the admitted root helper".to_string());
    }

    let admin = SurrealStorageIdentityAdminAdapter::new(
        parsed.required("--admin-user")?,
        Duration::from_secs(parsed.timeout_seconds),
        &spec,
    )?;
    let _lease = StorageIdentityEnrollmentLease::acquire()?;
    let admin_secret = SecretMaterial::read_human_admin_stdin()?;
    let secret_store = LocalStorageIdentitySecretAdapter;
    let report = enroll_storage_identity(&spec, &admin_secret, &admin, &secret_store);
    let failed = report.status != "completed";
    super::io::emit_json(&report, failed)
}

fn emit_preflight_plan(spec: &StorageIdentitySpec) -> Result<ExitCode, String> {
    let local =
        LocalStorageIdentitySecretAdapter.observe(&RuntimeSecretInstallSpec::storage_runtime());
    let local_state = if local.present {
        StorageEvidenceState::Present
    } else if local.unknowns.is_empty() {
        StorageEvidenceState::Absent
    } else {
        StorageEvidenceState::Unknown
    };
    let observation = StorageIdentityObservation {
        schema: STORAGE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
        identity_id: spec.identity_id.clone(),
        database_identity: StorageEvidenceState::Unknown,
        observed_role: None,
        local_credential: local_state,
        runtime_authentication: StorageEvidenceState::Unknown,
        admin_authority_present: false,
        secret_exposed: false,
        observed_at_ms: now_ms(),
        unknowns: vec!["database observation requires HumanAdmin authority on apply".to_string()],
    };
    super::io::emit_json(&reconcile_storage_identity(spec, &observation), false)
}

struct ParsedArgs {
    values: BTreeMap<String, String>,
    apply: bool,
    timeout_seconds: u64,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let admitted = [
            "--endpoint",
            "--namespace",
            "--database",
            "--admin-user",
            "--username",
            "--token-duration",
            "--session-duration",
            "--timeout-seconds",
        ];
        let mut values = BTreeMap::new();
        let mut apply = false;
        let mut index = 0;
        while index < args.len() {
            let flag = &args[index];
            if flag == "--apply" {
                if apply {
                    return Err("duplicate --apply flag".to_string());
                }
                apply = true;
                index += 1;
                continue;
            }
            if !admitted.contains(&flag.as_str()) {
                return Err(format!("storage-identity-enroll rejects argument: {flag}"));
            }
            let value = args
                .get(index + 1)
                .filter(|value| !value.starts_with("--"))
                .ok_or_else(|| format!("{flag} requires one value"))?;
            if values.insert(flag.clone(), value.clone()).is_some() {
                return Err(format!("duplicate {flag} flag"));
            }
            index += 2;
        }

        for required in ["--endpoint", "--namespace", "--database", "--admin-user"] {
            if !values.contains_key(required) {
                return Err(format!("storage-identity-enroll requires {required}"));
            }
        }
        let timeout_seconds =
            values
                .get("--timeout-seconds")
                .map_or(Ok(DEFAULT_TIMEOUT_SECONDS), |value| {
                    value
                        .parse::<u64>()
                        .map_err(|_| "--timeout-seconds must be an integer".to_string())
                })?;
        if !(1..=60).contains(&timeout_seconds) {
            return Err("--timeout-seconds must be between 1 and 60".to_string());
        }
        Ok(Self {
            values,
            apply,
            timeout_seconds,
        })
    }

    fn required(&self, flag: &str) -> Result<String, String> {
        self.values
            .get(flag)
            .cloned()
            .ok_or_else(|| format!("storage-identity-enroll requires {flag}"))
    }

    fn value_or(&self, flag: &str, fallback: &str) -> String {
        self.values
            .get(flag)
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    }

    fn spec(&self) -> StorageIdentitySpec {
        StorageIdentitySpec {
            schema: STORAGE_IDENTITY_SPEC_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            endpoint: self.values["--endpoint"].clone(),
            namespace: self.values["--namespace"].clone(),
            database: self.values["--database"].clone(),
            username: self.value_or("--username", DEFAULT_RUNTIME_USERNAME),
            role: StorageIdentityRole::Editor,
            token_duration: self.value_or("--token-duration", DEFAULT_TOKEN_DURATION),
            session_duration: self.value_or("--session-duration", DEFAULT_SESSION_DURATION),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parser_rejects_secret_flags_and_duplicates() {
        let secret = args(&[
            "--endpoint",
            "http://127.0.0.1:8000",
            "--namespace",
            "cynic",
            "--database",
            "main",
            "--admin-user",
            "root",
            "--admin-password",
            "secret",
        ]);
        assert!(ParsedArgs::parse(&secret).is_err());

        let duplicate = args(&[
            "--endpoint",
            "http://127.0.0.1:8000",
            "--namespace",
            "cynic",
            "--database",
            "main",
            "--admin-user",
            "root",
            "--admin-user",
            "other",
        ]);
        assert!(ParsedArgs::parse(&duplicate).is_err());
    }

    #[test]
    fn parser_builds_a_bounded_editor_identity() {
        let parsed = ParsedArgs::parse(&args(&[
            "--endpoint",
            "http://127.0.0.1:8000",
            "--namespace",
            "cynic",
            "--database",
            "main",
            "--admin-user",
            "root",
            "--apply",
        ]))
        .unwrap();
        let spec = parsed.spec();
        assert!(parsed.apply);
        assert_eq!(spec.username, DEFAULT_RUNTIME_USERNAME);
        assert_eq!(spec.role, StorageIdentityRole::Editor);
        assert_eq!(parsed.timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
    }
}
