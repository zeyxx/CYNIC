//! Pure types — no I/O, no policy. Shared vocabulary for build proof.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRequest {
    pub packages: Vec<String>,
    pub manifest: String,
    pub env: BuildEnv,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildEnv {
    pub admission_verified: bool,
    pub min_stack: Option<u64>,
    pub jobs: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub exit_code: i32,
    pub errors: ErrorCatalog,
    pub log_ref: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorCatalog { pub errors: Vec<ErrorEntry> }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry { pub code: String, pub file: String, pub message: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildProof {
    pub request: BuildRequest,
    pub result: BuildResult,
    pub builder: BuilderTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuilderTarget {
    pub host: String,
    pub workdir: String,
}
