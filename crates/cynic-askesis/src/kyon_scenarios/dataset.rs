use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{KyonScenario, admit_kyon_scenario};
use crate::AskesisError;

#[derive(Debug, Clone)]
pub struct KyonScenarioDataset {
    pub path: String,
    pub sha256: String,
    pub scenarios: Vec<KyonScenario>,
}

pub fn load_kyon_scenario_dataset(path: &Path) -> crate::Result<KyonScenarioDataset> {
    let raw = fs::read_to_string(path)?;
    let mut scenarios = Vec::new();

    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let scenario: KyonScenario = serde_json::from_str(line).map_err(|error| {
            AskesisError::KyonScenario(format!("{}:{}: {error}", path.display(), index + 1))
        })?;
        let admission = admit_kyon_scenario(&scenario);
        if admission.status != "admitted" {
            return Err(AskesisError::KyonScenario(format!(
                "{}: {}",
                scenario.id,
                admission.blockers.join(",")
            )));
        }
        scenarios.push(scenario);
    }

    if scenarios.is_empty() {
        return Err(AskesisError::KyonScenario(
            "dataset has no scenarios".to_string(),
        ));
    }

    Ok(KyonScenarioDataset {
        path: path.display().to_string(),
        sha256: format!("{:x}", Sha256::digest(raw.as_bytes())),
        scenarios,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn committed_kyon_scenarios_are_admitted() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("datasets")
            .join("kyon-scenarios-v0.jsonl");

        let dataset = load_kyon_scenario_dataset(&path).expect("load Kyon scenarios");

        assert_eq!(dataset.scenarios.len(), 6);
        assert!(!dataset.sha256.is_empty());
    }
}
