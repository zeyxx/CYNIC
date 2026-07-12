use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

const REQUIRED_CLASSES: &[&str] = &[
    "raw",
    "normalized",
    "warehouse",
    "ccm",
    "outbox",
    "deadletter",
    "cache",
    "artifact",
    "checkpoint",
    "local_journal",
    "local_snapshot",
    "desired_projection",
    "local_contract",
    "operator_view",
    "local_doc",
];

const REQUIRED_CLASSIFICATION_FIELDS: &[&str] = &[
    "path",
    "organ",
    "class",
    "authority",
    "backend",
    "promotion_path",
    "next_action",
];

const TRACKED_STATE_PATHS: &[&str] = &["organs/*/state/*", "infra/organ-anvil/*"];
const LOCAL_MEMORY_PREFIXES: &[&str] = &["organs/", "infra/organ-anvil/"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ContractFinding {
    pub severity: String,
    pub kind: String,
    pub path: String,
    pub detail: String,
}

impl ContractFinding {
    fn critical(kind: &str, path: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            severity: "critical".to_string(),
            kind: kind.to_string(),
            path: path.into(),
            detail: detail.into(),
        }
    }
}

pub fn validate_organ_memory_contract_value(
    contract: &Value,
    tracked_paths: &[String],
    contract_path: &str,
) -> Vec<ContractFinding> {
    let mut findings = Vec::new();

    if contract.get("schema").and_then(Value::as_str) != Some("cynic.organ_memory.v1") {
        findings.push(ContractFinding::critical(
            "unexpected_organ_memory_schema",
            contract_path,
            format!(
                "expected cynic.organ_memory.v1, got {:?}",
                contract.get("schema").and_then(Value::as_str)
            ),
        ));
    }

    let declared_classes: BTreeSet<&str> = contract
        .get("storage_classes")
        .and_then(Value::as_object)
        .map(|classes| classes.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let missing_classes: Vec<&str> = REQUIRED_CLASSES
        .iter()
        .copied()
        .filter(|class| !declared_classes.contains(class))
        .collect();
    if !missing_classes.is_empty() {
        findings.push(ContractFinding::critical(
            "organ_memory_classes_missing",
            contract_path,
            format!("missing storage classes: {}", missing_classes.join(", ")),
        ));
    }

    let authorities: BTreeSet<&str> = contract
        .get("authority_levels")
        .and_then(Value::as_object)
        .map(|levels| levels.keys().map(String::as_str).collect())
        .unwrap_or_default();

    let Some(classifications) = contract
        .get("state_path_classifications")
        .and_then(Value::as_array)
    else {
        findings.push(ContractFinding::critical(
            "organ_memory_classifications_missing",
            contract_path,
            "state_path_classifications must be a list of tracked local state paths and their memory role",
        ));
        return findings;
    };

    let mut classified_paths = BTreeSet::new();
    for entry in classifications {
        let Some(entry_obj) = entry.as_object() else {
            findings.push(ContractFinding::critical(
                "organ_memory_classification_invalid",
                contract_path,
                "every state_path_classifications entry must be an object",
            ));
            continue;
        };

        let path = entry_obj
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let missing_fields: Vec<&str> = REQUIRED_CLASSIFICATION_FIELDS
            .iter()
            .copied()
            .filter(|field| !entry_obj.get(*field).map(value_present).unwrap_or(false))
            .collect();
        if !missing_fields.is_empty() {
            findings.push(ContractFinding::critical(
                "organ_memory_classification_metadata_missing",
                if path.is_empty() {
                    contract_path.to_string()
                } else {
                    path.clone()
                },
                format!(
                    "classification is missing required fields: {}",
                    missing_fields.join(", ")
                ),
            ));
        }

        if !path.is_empty() && !classified_paths.insert(path.clone()) {
            findings.push(ContractFinding::critical(
                "organ_memory_path_classified_twice",
                path.clone(),
                "a local memory path must have exactly one classification",
            ));
        }

        let class = entry_obj
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !class.is_empty() && !declared_classes.contains(class) {
            findings.push(ContractFinding::critical(
                "organ_memory_unknown_class",
                path.clone(),
                format!("class {class:?} is not declared in storage_classes"),
            ));
        }

        let authority = entry_obj
            .get("authority")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !authority.is_empty() && !authorities.contains(authority) {
            findings.push(ContractFinding::critical(
                "organ_memory_unknown_authority",
                path.clone(),
                format!("authority {authority:?} is not declared in authority_levels"),
            ));
        }

        if LOCAL_MEMORY_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
            && authority == "organism_canonical"
        {
            findings.push(ContractFinding::critical(
                "local_memory_claims_global_authority",
                path.clone(),
                "organ-local state may buffer, compress, or project; it must not be organism-canonical",
            ));
        }
    }

    for path in tracked_paths {
        let name = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if matches!(name, "README.md" | "SKILL.md") {
            continue;
        }
        if !classified_paths.contains(path) {
            findings.push(ContractFinding::critical(
                "tracked_local_memory_without_classification",
                path.clone(),
                "tracked organ-local state must be classified as cache, journal, outbox, warehouse, artifact, or legacy debt",
            ));
        }
    }

    findings
}

pub fn validate_organ_memory_contract_at(project_root: &Path) -> Vec<ContractFinding> {
    let contract_path = project_root.join("axioms/organ-memory-v1.json");
    let contract_text = match std::fs::read_to_string(&contract_path) {
        Ok(text) => text,
        Err(e) => {
            return vec![ContractFinding::critical(
                "organ_memory_contract_missing",
                relative_or_display(project_root, &contract_path),
                format!(
                    "organ-memory-v1 must classify organ-local state before local stores can be trusted: {e}"
                ),
            )];
        }
    };

    let contract: Value = match serde_json::from_str(&contract_text) {
        Ok(value) => value,
        Err(e) => {
            return vec![ContractFinding::critical(
                "organ_memory_contract_invalid_json",
                relative_or_display(project_root, &contract_path),
                e.to_string(),
            )];
        }
    };

    let tracked_paths = git_ls_files(project_root, TRACKED_STATE_PATHS);
    validate_organ_memory_contract_value(
        &contract,
        &tracked_paths,
        &relative_or_display(project_root, &contract_path),
    )
}

fn git_ls_files(project_root: &Path, pathspecs: &[&str]) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .arg("ls-files")
        .args(pathspecs)
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(ToString::to_string)
        .collect()
}

fn relative_or_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(PathBuf::from)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn value_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_contract() -> Value {
        serde_json::json!({
            "schema": "cynic.organ_memory.v1",
            "storage_classes": {
                "raw": "raw",
                "normalized": "normalized",
                "warehouse": "warehouse",
                "ccm": "ccm",
                "outbox": "outbox",
                "deadletter": "deadletter",
                "cache": "cache",
                "artifact": "artifact",
                "checkpoint": "checkpoint",
                "local_journal": "local_journal",
                "local_snapshot": "local_snapshot",
                "desired_projection": "desired_projection",
                "local_contract": "local_contract",
                "operator_view": "operator_view",
                "local_doc": "local_doc"
            },
            "authority_levels": {
                "local_only": "local only",
                "kernel_promotion_candidate": "promotable",
                "derived_from_git_or_kernel": "projection",
                "legacy_debt": "legacy"
            },
            "state_path_classifications": [{
                "path": "organs/forge/state/state.json",
                "organ": "organ-forge",
                "class": "local_snapshot",
                "authority": "legacy_debt",
                "backend": "json",
                "promotion_path": "cynic_observe",
                "next_action": "move to untracked state"
            }]
        })
    }

    #[test]
    fn accepts_classified_tracked_state() {
        let findings = validate_organ_memory_contract_value(
            &base_contract(),
            &["organs/forge/state/state.json".to_string()],
            "axioms/organ-memory-v1.json",
        );

        assert!(
            findings
                .iter()
                .all(|finding| finding.severity != "critical")
        );
    }

    #[test]
    fn rejects_unclassified_tracked_state() {
        let findings = validate_organ_memory_contract_value(
            &base_contract(),
            &["organs/forge/state/audit.jsonl".to_string()],
            "axioms/organ-memory-v1.json",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.kind == "tracked_local_memory_without_classification")
        );
    }

    #[test]
    fn rejects_local_memory_global_authority() {
        let mut contract = base_contract();
        contract["authority_levels"]["organism_canonical"] =
            Value::String("forbidden for local state".to_string());
        contract["state_path_classifications"][0]["authority"] =
            Value::String("organism_canonical".to_string());

        let findings = validate_organ_memory_contract_value(
            &contract,
            &["organs/forge/state/state.json".to_string()],
            "axioms/organ-memory-v1.json",
        );

        assert!(
            findings
                .iter()
                .any(|finding| finding.kind == "local_memory_claims_global_authority")
        );
    }
}
