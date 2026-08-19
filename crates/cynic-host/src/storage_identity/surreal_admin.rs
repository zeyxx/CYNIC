use std::time::Duration;
use std::{cmp, io::Read};

use cynic_protocol::{
    StorageEvidenceState, StorageIdentityRole, StorageIdentitySpec, storage_identity_spec_blockers,
};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::Deserialize;
use zeroize::Zeroize;

use crate::secret_material::SecretMaterial;

use super::{StorageAdminObservation, StorageIdentityAdminPort};

#[derive(Debug, Clone)]
pub(crate) struct SurrealStorageIdentityAdminAdapter {
    client: Client,
    admin_username: String,
}

#[derive(Debug, Deserialize)]
struct StatementResponse {
    status: Option<String>,
    result: Option<serde_json::Value>,
}

struct SensitiveBody {
    bytes: Vec<u8>,
    offset: usize,
}

impl SensitiveBody {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, offset: 0 }
    }
}

impl Read for SensitiveBody {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let count = cmp::min(output.len(), self.bytes.len().saturating_sub(self.offset));
        output[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }
}

impl Drop for SensitiveBody {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

impl SurrealStorageIdentityAdminAdapter {
    pub(crate) fn new(
        admin_username: impl Into<String>,
        timeout: Duration,
        spec: &StorageIdentitySpec,
    ) -> Result<Self, String> {
        let admin_username = admin_username.into();
        if !valid_identifier(&admin_username) {
            return Err("storage admin username is invalid".to_string());
        }
        validate_spec(spec)?;
        let client = Client::builder()
            .timeout(timeout)
            .redirect(Policy::none())
            .no_proxy()
            .build()
            .map_err(|error| format!("storage admin HTTP client unavailable: {error}"))?;
        Ok(Self {
            client,
            admin_username,
        })
    }

    fn execute(
        &self,
        spec: &StorageIdentitySpec,
        username: &str,
        password: &SecretMaterial,
        sql: SensitiveBody,
    ) -> Result<Vec<StatementResponse>, String> {
        validate_spec(spec)?;
        let endpoint = spec.endpoint.trim_end_matches('/');
        let response = self
            .client
            .post(format!("{endpoint}/sql"))
            .basic_auth(username, Some(password.expose_str()))
            .header("Accept", "application/json")
            .header("surreal-ns", &spec.namespace)
            .header("surreal-db", &spec.database)
            .body(reqwest::blocking::Body::new(sql))
            .send()
            .map_err(|error| format!("storage identity HTTP request failed: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("storage identity HTTP status {status}"));
        }
        let statements = response
            .json::<Vec<StatementResponse>>()
            .map_err(|error| format!("storage identity response parse failed: {error}"))?;
        if statements.is_empty()
            || statements
                .iter()
                .any(|statement| statement.status.as_deref() != Some("OK"))
        {
            return Err("storage identity statement was not positively acknowledged".to_string());
        }
        Ok(statements)
    }

    pub(crate) fn migrate_schema(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
        sql: Vec<u8>,
    ) -> Result<serde_json::Value, String> {
        if sql.is_empty() || sql.len() > 2 * 1024 * 1024 {
            return Err("storage schema payload size is not admitted".to_string());
        }
        let uppercase = String::from_utf8_lossy(&sql).to_ascii_uppercase();
        if ["DEFINE USER", "REMOVE USER", " PASSWORD ", "DEFINE ACCESS"]
            .iter()
            .any(|marker| uppercase.contains(marker))
        {
            return Err("storage schema payload attempts to own identity authority".to_string());
        }
        self.execute(
            spec,
            &self.admin_username,
            admin_secret,
            SensitiveBody::new(sql),
        )?;
        let observed = self.execute(
            spec,
            &self.admin_username,
            admin_secret,
            SensitiveBody::new(b"INFO FOR DB;".to_vec()),
        )?;
        observed
            .into_iter()
            .filter_map(|statement| statement.result)
            .find(|result| result.get("tables").is_some())
            .ok_or_else(|| "storage schema read-back omitted table definitions".to_string())
    }

    fn observe_with(
        &self,
        spec: &StorageIdentitySpec,
        username: &str,
        password: &SecretMaterial,
    ) -> StorageAdminObservation {
        let statements = match self.execute(
            spec,
            username,
            password,
            SensitiveBody::new(b"INFO FOR DB;".to_vec()),
        ) {
            Ok(statements) => statements,
            Err(error) => {
                return StorageAdminObservation {
                    database_identity: StorageEvidenceState::Unknown,
                    role: None,
                    unknowns: vec![error],
                };
            }
        };
        let definition = statements
            .iter()
            .filter_map(|statement| statement.result.as_ref())
            .find_map(|result| result.get("users"))
            .and_then(|users| users.get(&spec.username))
            .and_then(|value| value.as_str());
        match definition {
            Some(definition) => {
                let role = parse_role(definition);
                StorageAdminObservation {
                    database_identity: StorageEvidenceState::Present,
                    role,
                    unknowns: if role.is_none() {
                        vec!["storage_identity_role_unrecognized".to_string()]
                    } else {
                        Vec::new()
                    },
                }
            }
            None => StorageAdminObservation {
                database_identity: StorageEvidenceState::Absent,
                role: None,
                unknowns: Vec::new(),
            },
        }
    }
}

impl StorageIdentityAdminPort for SurrealStorageIdentityAdminAdapter {
    fn observe(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> StorageAdminObservation {
        self.observe_with(spec, &self.admin_username, admin_secret)
    }

    fn create(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String> {
        validate_spec(spec)?;
        let sql = format!(
            "DEFINE USER `{}` ON DATABASE PASSWORD '{}' ROLES EDITOR DURATION FOR TOKEN {}, FOR SESSION {};",
            spec.username,
            runtime_secret.expose_str(),
            spec.token_duration,
            spec.session_duration
        )
        .into_bytes();
        self.execute(
            spec,
            &self.admin_username,
            admin_secret,
            SensitiveBody::new(sql),
        )?;
        Ok(())
    }

    fn remove(
        &self,
        spec: &StorageIdentitySpec,
        admin_secret: &SecretMaterial,
    ) -> Result<(), String> {
        validate_spec(spec)?;
        let sql = format!("REMOVE USER `{}` ON DATABASE;", spec.username).into_bytes();
        self.execute(
            spec,
            &self.admin_username,
            admin_secret,
            SensitiveBody::new(sql),
        )?;
        let observed = self.observe(spec, admin_secret);
        if observed.database_identity == StorageEvidenceState::Absent {
            Ok(())
        } else {
            Err("database identity compensation read-back failed".to_string())
        }
    }

    fn verify_runtime(
        &self,
        spec: &StorageIdentitySpec,
        runtime_secret: &SecretMaterial,
    ) -> Result<(), String> {
        let statements = self.execute(
            spec,
            &spec.username,
            runtime_secret,
            SensitiveBody::new(b"RETURN true;".to_vec()),
        )?;
        if statements
            .first()
            .and_then(|statement| statement.result.as_ref())
            == Some(&serde_json::Value::Bool(true))
        {
            Ok(())
        } else {
            Err("runtime identity authentication proof was not true".to_string())
        }
    }
}

fn validate_spec(spec: &StorageIdentitySpec) -> Result<(), String> {
    if !storage_identity_spec_blockers(spec).is_empty() || !valid_endpoint(&spec.endpoint) {
        return Err("storage identity spec is not admitted by the SurrealDB adapter".to_string());
    }
    Ok(())
}

fn valid_endpoint(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !matches!(url.path(), "" | "/")
    {
        return false;
    }
    match url.scheme() {
        "https" => url.host_str().is_some(),
        // Plaintext admin auth is admitted on loopback only. Parse the typed
        // host rather than compare strings: `host_str()` returns "[::1]" *with*
        // brackets for IPv6, so matching against "::1" silently rejected the
        // IPv6 loopback.
        "http" => url
            .host_str()
            .map(|host| host.trim_start_matches('[').trim_end_matches(']'))
            .and_then(|host| host.parse::<std::net::IpAddr>().ok())
            .is_some_and(|addr| addr.is_loopback()),
        _ => false,
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn parse_role(definition: &str) -> Option<StorageIdentityRole> {
    definition
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .any(|pair| pair == ["ROLES", "EDITOR"])
        .then_some(StorageIdentityRole::Editor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_and_durations_reject_surrealql_injection() {
        assert!(valid_identifier("cynic-runtime"));
        assert!(!valid_identifier("cynic`; REMOVE USER root"));
    }

    #[test]
    fn plaintext_admin_auth_is_only_admitted_on_loopback() {
        assert!(valid_endpoint("http://127.0.0.1:8000"));
        assert!(valid_endpoint("http://[::1]:8000"));
        assert!(valid_endpoint("https://surreal.internal:8000"));
        assert!(!valid_endpoint("http://localhost:8000"));
        assert!(!valid_endpoint("http://10.10.10.101:8000"));
        assert!(!valid_endpoint("https://user:pass@surreal.internal:8000"));
        assert!(!valid_endpoint("https://surreal.internal:8000/sql"));
    }

    #[test]
    fn plaintext_admin_auth_refuses_every_non_loopback_address() {
        // The whole 127.0.0.0/8 block is loopback, so accept it; anything
        // routable over the wire must stay refused for plaintext auth.
        assert!(valid_endpoint("http://127.0.0.2:8000"));
        assert!(!valid_endpoint("http://0.0.0.0:8000"));
        assert!(!valid_endpoint("http://192.168.100.1:8000"));
        assert!(!valid_endpoint("http://[fe80::1]:8000"));
        assert!(!valid_endpoint("http://[2001:db8::1]:8000"));
    }

    #[test]
    fn role_parser_accepts_only_editor_read_back() {
        assert_eq!(
            parse_role("DEFINE USER `cynic-runtime` ON DATABASE ROLES EDITOR"),
            Some(StorageIdentityRole::Editor)
        );
        assert_eq!(
            parse_role("DEFINE USER `cynic-runtime` ON DATABASE ROLES OWNER"),
            None
        );
    }
}
