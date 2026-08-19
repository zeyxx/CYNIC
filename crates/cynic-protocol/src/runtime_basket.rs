use std::{collections::BTreeSet, path::Path};

use serde::{Deserialize, Serialize};

pub const RUNTIME_BASKET_SCHEMA: &str = "cynic.runtime_basket.v1";

const RUNTIME_BASKET_DOCUMENT: &str = include_str!("../../../infra/runtime-basket.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBasketDesiredState {
    pub schema: String,
    pub status: String,
    pub runtime_seed: RuntimeSeedDeclaration,
    pub artifacts: Vec<RuntimeBasketArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSeedDeclaration {
    pub id: String,
    pub binary: String,
    pub install_path: String,
    pub contract_revision: u64,
    pub signature_namespace: String,
    pub runtime_basket_member: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBasketArtifact {
    pub id: String,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub binary: String,
    pub install_path: String,
    pub roles: Vec<String>,
}

pub fn runtime_basket_desired_state() -> Result<RuntimeBasketDesiredState, String> {
    let basket: RuntimeBasketDesiredState = serde_json::from_str(RUNTIME_BASKET_DOCUMENT)
        .map_err(|error| format!("runtime_basket_decode_failed:{error}"))?;
    let blockers = validate_runtime_basket(&basket);
    if blockers.is_empty() {
        Ok(basket)
    } else {
        Err(blockers.join(","))
    }
}

pub fn runtime_release_artifact_names() -> Result<Vec<String>, String> {
    let mut names = runtime_basket_desired_state()?
        .artifacts
        .into_iter()
        .map(|artifact| artifact.binary)
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

pub fn validate_runtime_basket(basket: &RuntimeBasketDesiredState) -> Vec<String> {
    let mut blockers = Vec::new();
    if basket.schema != RUNTIME_BASKET_SCHEMA || basket.status != "normative" {
        blockers.push("runtime_basket_authority_invalid".to_string());
    }
    if basket.runtime_seed.runtime_basket_member
        || basket.runtime_seed.contract_revision == 0
        || !valid_component(&basket.runtime_seed.binary)
        || !valid_install_path(
            &basket.runtime_seed.install_path,
            &basket.runtime_seed.binary,
        )
    {
        blockers.push("runtime_basket_seed_boundary_invalid".to_string());
    }
    if basket.artifacts.is_empty() {
        blockers.push("runtime_basket_artifacts_missing".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut binaries = BTreeSet::new();
    let mut install_paths = BTreeSet::new();
    for artifact in &basket.artifacts {
        if !valid_component(&artifact.id)
            || !valid_component(&artifact.crate_name)
            || !valid_component(&artifact.binary)
            || artifact.roles.is_empty()
            || artifact.roles.iter().any(|role| !valid_component(role))
            || !valid_install_path(&artifact.install_path, &artifact.binary)
            || artifact.binary == basket.runtime_seed.binary
        {
            blockers.push(format!("runtime_basket_artifact_invalid:{}", artifact.id));
        }
        if !ids.insert(&artifact.id)
            || !binaries.insert(&artifact.binary)
            || !install_paths.insert(&artifact.install_path)
        {
            blockers.push(format!("runtime_basket_artifact_duplicate:{}", artifact.id));
        }
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_install_path(value: &str, binary: &str) -> bool {
    let path = Path::new(value);
    path.is_absolute()
        && !value.contains(['\n', '\r'])
        && path.file_name().and_then(|name| name.to_str()) == Some(binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_basket_drives_the_complete_mutable_release_set() {
        let basket = runtime_basket_desired_state().expect("valid canonical runtime basket");
        let names = runtime_release_artifact_names().expect("release artifacts");

        assert!(names.contains(&"cynic-mcp".to_string()));
        assert!(names.contains(&"cynic-lxc-exec-bridge".to_string()));
        assert!(!names.contains(&basket.runtime_seed.binary));
    }

    #[test]
    fn seed_cannot_amplify_itself_into_the_mutable_basket() {
        let mut basket = runtime_basket_desired_state().expect("valid canonical runtime basket");
        basket.runtime_seed.runtime_basket_member = true;

        assert!(
            validate_runtime_basket(&basket)
                .contains(&"runtime_basket_seed_boundary_invalid".to_string())
        );
    }
}
