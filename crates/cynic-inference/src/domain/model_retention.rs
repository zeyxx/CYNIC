//! Pure retention assessment for model artifacts.
//!
//! This is an evidence projection over the canonical model store. It never
//! deletes, moves, or assumes that inactivity proves safe removal.

use serde::{Deserialize, Serialize};

use super::model_store::{ModelArtifact, ModelLocationKind, ModelStoreReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRetentionClass {
    ProtectedByProfile,
    ReconstructibleCandidate,
    CorruptArtifact,
    UsageProofRequired,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRetentionAssessment {
    pub schema: String,
    pub artifact_id: String,
    pub path: String,
    pub class: ModelRetentionClass,
    pub reasons: Vec<String>,
    pub deletion_authorized: bool,
}

pub fn assess_artifact(artifact: &ModelArtifact) -> ModelRetentionAssessment {
    let (class, reasons) = if !artifact.exists {
        (
            ModelRetentionClass::Missing,
            vec!["artifact path is missing".to_string()],
        )
    } else if artifact.path.to_ascii_lowercase().contains(".corrupt") {
        (
            ModelRetentionClass::CorruptArtifact,
            vec!["filename declares corrupt artifact".to_string()],
        )
    } else if artifact.profile_alias.is_some() {
        (
            ModelRetentionClass::ProtectedByProfile,
            vec!["linked to a declared runtime profile".to_string()],
        )
    } else if matches!(
        &artifact.location,
        ModelLocationKind::HfCache | ModelLocationKind::LlamaCache
    ) {
        (
            ModelRetentionClass::ReconstructibleCandidate,
            vec!["located in a reconstructible model cache".to_string()],
        )
    } else {
        (
            ModelRetentionClass::UsageProofRequired,
            vec!["no profile or reconstructible-cache provenance observed".to_string()],
        )
    };

    ModelRetentionAssessment {
        schema: "cynic.inference.model_retention_assessment.v1".to_string(),
        artifact_id: artifact.id.clone(),
        path: artifact.path.clone(),
        class,
        reasons,
        deletion_authorized: false,
    }
}

pub fn assess_report(report: &ModelStoreReport) -> Vec<ModelRetentionAssessment> {
    report.artifacts.iter().map(assess_artifact).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(location: ModelLocationKind, path: &str) -> ModelArtifact {
        ModelArtifact {
            id: "model".to_string(),
            path: path.to_string(),
            location,
            kind: "gguf".to_string(),
            size_bytes: Some(10),
            exists: true,
            profile_alias: None,
            hf_repo: None,
            hf_revision: None,
        }
    }

    #[test]
    fn corrupt_artifact_is_not_silently_delete_authorized() {
        let assessment = assess_artifact(&artifact(
            ModelLocationKind::Vault,
            "/models/Qwen3.gguf.corrupt",
        ));
        assert_eq!(assessment.class, ModelRetentionClass::CorruptArtifact);
        assert!(!assessment.deletion_authorized);
    }

    #[test]
    fn cache_without_usage_proof_is_only_a_candidate() {
        let assessment = assess_artifact(&artifact(
            ModelLocationKind::LlamaCache,
            "/cache/model.gguf",
        ));
        assert_eq!(
            assessment.class,
            ModelRetentionClass::ReconstructibleCandidate
        );
    }
}
