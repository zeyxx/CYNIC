use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSecretClass {
    RuntimeMachineIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSecretPresenceSource {
    ProcessEnvironment,
    PrincipalOwnedFile,
    InstalledFile,
}

#[derive(Clone)]
pub struct RuntimeSecretPresenceSpec {
    identity_id: String,
    required_class: RuntimeSecretClass,
    environment_keys: Vec<String>,
    file_candidates: Vec<(PathBuf, RuntimeSecretPresenceSource)>,
}

impl RuntimeSecretPresenceSpec {
    pub fn storage_runtime() -> Self {
        Self {
            identity_id: "storage_runtime".to_string(),
            required_class: RuntimeSecretClass::RuntimeMachineIdentity,
            environment_keys: vec!["SURREALDB_PASS".to_string()],
            file_candidates: vec![(
                PathBuf::from("/etc/cynic/secrets/storage-runtime-password"),
                RuntimeSecretPresenceSource::InstalledFile,
            )],
        }
    }

    pub fn proxmox_control() -> Self {
        Self {
            identity_id: "proxmox_control".to_string(),
            required_class: RuntimeSecretClass::RuntimeMachineIdentity,
            environment_keys: vec![
                "CYNIC_PROXMOX_TOKEN_SECRET".to_string(),
                "CYNIC_PROXMOX_SECRET".to_string(),
            ],
            file_candidates: vec![(
                PathBuf::from("/etc/cynic/secrets/proxmox-token"),
                RuntimeSecretPresenceSource::InstalledFile,
            )],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeSecretPresenceReport {
    pub schema: &'static str,
    pub identity_id: String,
    pub required_class: RuntimeSecretClass,
    pub present: bool,
    pub source: Option<RuntimeSecretPresenceSource>,
    pub secret_exposed: bool,
    pub unknowns: Vec<String>,
}

pub fn observe_runtime_secret_presence(
    spec: &RuntimeSecretPresenceSpec,
) -> RuntimeSecretPresenceReport {
    if spec
        .environment_keys
        .iter()
        .any(|key| std::env::var_os(key).is_some_and(|value| !value.as_os_str().is_empty()))
    {
        return report(
            spec,
            true,
            Some(RuntimeSecretPresenceSource::ProcessEnvironment),
            Vec::new(),
        );
    }

    let mut unknowns = Vec::new();
    for (path, source) in &spec.file_candidates {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() && metadata.len() > 0 => {
                return report(spec, true, Some(*source), unknowns);
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                unknowns.push("runtime_secret_candidate_is_symlink".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => unknowns.push("runtime_secret_metadata_unavailable".to_string()),
        }
    }

    report(spec, false, None, unknowns)
}

fn report(
    spec: &RuntimeSecretPresenceSpec,
    present: bool,
    source: Option<RuntimeSecretPresenceSource>,
    mut unknowns: Vec<String>,
) -> RuntimeSecretPresenceReport {
    unknowns.sort();
    unknowns.dedup();
    RuntimeSecretPresenceReport {
        schema: "cynic.runtime_secret_presence.v1",
        identity_id: spec.identity_id.clone(),
        required_class: spec.required_class,
        present,
        source,
        secret_exposed: false,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_identity_exposes_no_locator_or_material() {
        let spec = RuntimeSecretPresenceSpec {
            identity_id: "fixture".to_string(),
            required_class: RuntimeSecretClass::RuntimeMachineIdentity,
            environment_keys: Vec::new(),
            file_candidates: Vec::new(),
        };

        let report = observe_runtime_secret_presence(&spec);
        let json = serde_json::to_string(&report).expect("serialize report");

        assert!(!report.present);
        assert!(!report.secret_exposed);
        assert!(!json.contains("/etc/"));
        assert!(!json.contains("TOKEN"));
        assert!(!json.contains("commitment"));
        assert!(!json.contains("length"));
        assert!(!json.contains("shape"));
    }
}
