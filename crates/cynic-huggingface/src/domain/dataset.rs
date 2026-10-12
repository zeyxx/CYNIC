use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HfDatasetSpec {
    pub repo: String,
    pub subset: Option<String>,
    pub revision: Option<String>,
    pub format_version: u16,
}

impl HfDatasetSpec {
    pub fn canonical_repo_kind(&self) -> &'static str {
        "dataset"
    }
}