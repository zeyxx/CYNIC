use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::AskesisError;
use crate::eval::policy::validate_policy;
use crate::eval::types::{AgentEvalDataset, EvalCase};

pub fn load_jsonl_dataset(path: &Path) -> crate::Result<AgentEvalDataset> {
    let raw = fs::read_to_string(path)?;
    let mut cases = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let case: EvalCase = serde_json::from_str(line).map_err(|error| {
            AskesisError::EvalDataset(format!("{}:{}: {error}", path.display(), index + 1))
        })?;
        if case.schema != "cynic.askesis.agent_eval.case.v1" {
            return Err(AskesisError::EvalDataset(format!(
                "{}: unsupported schema {}",
                case.id, case.schema
            )));
        }
        if case.id.trim().is_empty() || case.prompt.trim().is_empty() {
            return Err(AskesisError::EvalDataset(format!(
                "line {} has empty id or prompt",
                index + 1
            )));
        }
        validate_policy(&case.policy)?;
        cases.push(case);
    }
    if cases.is_empty() {
        return Err(AskesisError::EvalDataset("dataset has no cases".into()));
    }
    let sha256 = format!("{:x}", Sha256::digest(raw.as_bytes()));
    Ok(AgentEvalDataset {
        path: path.display().to_string(),
        sha256,
        cases,
    })
}
