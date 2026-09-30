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
    pub identity_id: String,
    pub required_class: RuntimeSecretClass,
    pub environment_keys: Vec<String>,
    pub file_candidates: Vec<(PathBuf, RuntimeSecretPresenceSource)>,
}

impl RuntimeSecretPresenceSpec {
    pub fn new(id: &str, env_keys: &[&str], filename: &str) -> Self {
        let mut file_candidates = Vec::new();
        if let Ok(home) = std::env::var("HOME") {
            file_candidates.push((
                PathBuf::from(home).join(".config/cynic/secrets").join(filename),
                RuntimeSecretPresenceSource::PrincipalOwnedFile,
            ));
        }
        file_candidates.push((
            PathBuf::from("/etc/cynic/secrets").join(filename),
            RuntimeSecretPresenceSource::InstalledFile,
        ));
        Self {
            identity_id: id.to_string(),
            required_class: RuntimeSecretClass::RuntimeMachineIdentity,
            environment_keys: env_keys.iter().map(|k| k.to_string()).collect(),
            file_candidates,
        }
    }

    pub fn storage_runtime() -> Self {
        Self::new("storage_runtime", &["SURREALDB_PASS"], "storage-runtime-password")
    }

    pub fn proxmox_control() -> Self {
        Self::new(
            "proxmox_control",
            &["CYNIC_PROXMOX_TOKEN_SECRET", "CYNIC_PROXMOX_SECRET"],
            "proxmox-token",
        )
    }

    pub fn cloudflare() -> Self {
        Self::new("cloudflare", &["CLOUDFLARE_API_TOKEN"], "cloudflare-api-token")
    }

    pub fn porkbun() -> Self {
        Self::new(
            "porkbun",
            &["PORKBUN_API_KEY", "PORKBUN_SECRET_KEY"],
            "porkbun-api-key",
        )
    }

    pub fn forgejo() -> Self {
        Self::new(
            "forgejo",
            &["FORGEJO_TOKEN", "CYNIC_FORGEJO_TOKEN"],
            "forgejo-token",
        )
    }

    pub fn huggingface() -> Self {
        Self::new("huggingface", &["HF_TOKEN"], "hf-token")
    }

    pub fn openai() -> Self {
        Self::new("openai", &["OPENAI_API_KEY"], "openai-api-key")
    }

    pub fn anthropic() -> Self {
        Self::new("anthropic", &["ANTHROPIC_API_KEY"], "anthropic-api-key")
    }

    pub fn all_runtime_specs() -> Vec<Self> {
        vec![
            Self::storage_runtime(),
            Self::proxmox_control(),
            Self::cloudflare(),
            Self::porkbun(),
            Self::forgejo(),
            Self::huggingface(),
            Self::openai(),
            Self::anthropic(),
        ]
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
