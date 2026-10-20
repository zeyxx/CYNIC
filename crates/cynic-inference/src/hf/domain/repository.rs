use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HfRepositoryRef {
    pub namespace: String,
    pub repository: String,
    /// Optional immutable revision. `None` means a remote HEAD resolution request.
    pub revision: Option<String>,
}

impl HfRepositoryRef {
    pub fn validated(
        namespace: impl Into<String>,
        repository: impl Into<String>,
        revision: Option<String>,
    ) -> Result<Self, HfBoundaryError> {
        let namespace = namespace.into();
        let repository = repository.into();
        if namespace.trim().is_empty() {
            return Err(HfBoundaryError::EmptyNamespace);
        }
        if repository.trim().is_empty() {
            return Err(HfBoundaryError::EmptyRepository);
        }
        if let Some(rev) = &revision {
            if rev.trim().is_empty() {
                return Err(HfBoundaryError::EmptyRevision);
            }
        }
        Ok(Self {
            namespace,
            repository,
            revision,
        })
    }

    pub fn slug(&self) -> String {
        format!("{}/{}", self.namespace, self.repository)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HfBoundaryError {
    #[error("namespace required")]
    EmptyNamespace,
    #[error("repository required")]
    EmptyRepository,
    #[error("revision required when explicitly supplied")]
    EmptyRevision,
}

/// Repository families hosted on the Hub. Maps 1:1 to `hf --type [...]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HfRepoType {
    Model,
    Dataset,
    Space,
}

impl HfRepoType {
    pub fn as_arg(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::Dataset => "dataset",
            Self::Space => "space",
        }
    }
}
