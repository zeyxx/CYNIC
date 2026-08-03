use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::Duration;

use cynic_protocol::{STORAGE_IDENTITY_SPEC_SCHEMA, StorageIdentityRole, StorageIdentitySpec};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::secret_material::SecretMaterial;
use crate::storage_identity::SurrealStorageIdentityAdminAdapter;

const MAX_TIMEOUT_SECONDS: u64 = 120;

#[derive(Serialize)]
struct StorageSchemaMigrationReport {
    schema: &'static str,
    status: &'static str,
    mutates: bool,
    requested_apply: bool,
    schema_sha256: String,
    schema_bytes: u64,
    required_tables: Vec<&'static str>,
    proofs: Vec<String>,
    blockers: Vec<String>,
    secret_exposed: bool,
}

pub(crate) fn execute(args: &[String]) -> Result<ExitCode, String> {
    let parsed = ParsedArgs::parse(args)?;
    let bytes = std::fs::read(&parsed.schema_file)
        .map_err(|error| format!("storage schema file unavailable: {error}"))?;
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(&bytes)));
    let schema_bytes = bytes.len() as u64;
    if digest != parsed.expected_sha256 {
        return emit(report(
            "blocked",
            parsed.apply,
            digest,
            schema_bytes,
            Vec::new(),
            vec!["schema_digest_mismatch".to_string()],
        ));
    }
    if !parsed.apply {
        return emit(report(
            "planned",
            false,
            digest,
            schema_bytes,
            vec!["content_addressed_schema_preflight".to_string()],
            Vec::new(),
        ));
    }
    if unsafe { libc::geteuid() } != 0 {
        return Err("storage schema migration requires the admitted root helper".to_string());
    }
    let spec = parsed.spec();
    let admin = SurrealStorageIdentityAdminAdapter::new(
        parsed.required("--admin-user")?,
        Duration::from_secs(parsed.timeout_seconds),
        &spec,
    )?;
    let admin_secret = SecretMaterial::read_human_admin_stdin()?;
    let info = admin.migrate_schema(&spec, &admin_secret, bytes)?;
    let blockers = schema_blockers(&info);
    let completed = blockers.is_empty();
    emit(report(
        if completed { "completed" } else { "blocked" },
        true,
        digest,
        schema_bytes,
        if completed {
            vec![
                "agent_workload_delivery_schemafull_read_back".to_string(),
                "agent_workspace_claim_schemafull_read_back".to_string(),
                "runtime_proof_commit_schemafull_read_back".to_string(),
            ]
        } else {
            Vec::new()
        },
        blockers,
    ))
}

fn schema_blockers(info: &serde_json::Value) -> Vec<String> {
    let tables = info.get("tables").and_then(serde_json::Value::as_object);
    [
        "agent_workload_delivery",
        "agent_workspace_claim",
        "runtime_proof_commit",
    ]
    .into_iter()
    .filter_map(|table| {
        let definition = tables
            .and_then(|values| values.get(table))
            .and_then(serde_json::Value::as_str);
        match definition {
            Some(value) if value.contains("SCHEMAFULL") => None,
            Some(_) => Some(format!("table_not_schemafull:{table}")),
            None => Some(format!("table_missing:{table}")),
        }
    })
    .collect()
}

fn report(
    status: &'static str,
    requested_apply: bool,
    schema_sha256: String,
    schema_bytes: u64,
    proofs: Vec<String>,
    blockers: Vec<String>,
) -> StorageSchemaMigrationReport {
    StorageSchemaMigrationReport {
        schema: "cynic.storage_schema_migration_report.v1",
        status,
        mutates: requested_apply,
        requested_apply,
        schema_sha256,
        schema_bytes,
        required_tables: vec![
            "agent_workload_delivery",
            "agent_workspace_claim",
            "runtime_proof_commit",
        ],
        proofs,
        blockers,
        secret_exposed: false,
    }
}

fn emit(report: StorageSchemaMigrationReport) -> Result<ExitCode, String> {
    let failed = matches!(report.status, "blocked" | "failed");
    super::io::emit_json(&report, failed)
}

struct ParsedArgs {
    values: BTreeMap<String, String>,
    apply: bool,
    timeout_seconds: u64,
    schema_file: String,
    expected_sha256: String,
}

impl ParsedArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let admitted = [
            "--endpoint",
            "--namespace",
            "--database",
            "--admin-user",
            "--schema-file",
            "--expected-sha256",
            "--timeout-seconds",
        ];
        let mut values = BTreeMap::new();
        let mut apply = false;
        let mut index = 0;
        while index < args.len() {
            if args[index] == "--apply" {
                if apply {
                    return Err("duplicate --apply flag".to_string());
                }
                apply = true;
                index += 1;
                continue;
            }
            let flag = &args[index];
            if !admitted.contains(&flag.as_str()) {
                return Err(format!("storage-schema-migrate rejects argument: {flag}"));
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
        for required in [
            "--endpoint",
            "--namespace",
            "--database",
            "--admin-user",
            "--schema-file",
            "--expected-sha256",
        ] {
            if !values.contains_key(required) {
                return Err(format!("storage-schema-migrate requires {required}"));
            }
        }
        let schema_file = values["--schema-file"].clone();
        if !schema_file.starts_with('/') || schema_file.contains("..") {
            return Err("schema file must be an absolute normalized path".to_string());
        }
        let expected_sha256 = values["--expected-sha256"].clone();
        if !valid_sha256(&expected_sha256) {
            return Err("expected schema digest must be sha256:<64 lowercase hex>".to_string());
        }
        let timeout_seconds = values
            .get("--timeout-seconds")
            .map_or(Ok(MAX_TIMEOUT_SECONDS), |value| value.parse::<u64>())
            .map_err(|_| "timeout must be an integer".to_string())?;
        if !(1..=MAX_TIMEOUT_SECONDS).contains(&timeout_seconds) {
            return Err(format!(
                "timeout must be between 1 and {MAX_TIMEOUT_SECONDS}"
            ));
        }
        Ok(Self {
            values,
            apply,
            timeout_seconds,
            schema_file,
            expected_sha256,
        })
    }

    fn required(&self, flag: &str) -> Result<String, String> {
        self.values
            .get(flag)
            .cloned()
            .ok_or_else(|| format!("storage-schema-migrate requires {flag}"))
    }

    fn spec(&self) -> StorageIdentitySpec {
        StorageIdentitySpec {
            schema: STORAGE_IDENTITY_SPEC_SCHEMA.to_string(),
            identity_id: "storage-schema-migrator".to_string(),
            endpoint: self.values["--endpoint"].clone(),
            namespace: self.values["--namespace"].clone(),
            database: self.values["--database"].clone(),
            username: "storage-schema-migrator".to_string(),
            role: StorageIdentityRole::Editor,
            token_duration: "5m".to_string(),
            session_duration: "15m".to_string(),
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_read_back_requires_exact_schemafull_tables() {
        let info = serde_json::json!({"tables": {
            "agent_workload_delivery": "DEFINE TABLE agent_workload_delivery SCHEMAFULL",
            "agent_workspace_claim": "DEFINE TABLE agent_workspace_claim SCHEMALESS",
            "runtime_proof_commit": "DEFINE TABLE runtime_proof_commit SCHEMAFULL"
        }});
        assert_eq!(
            schema_blockers(&info),
            vec!["table_not_schemafull:agent_workspace_claim"]
        );
    }
}
