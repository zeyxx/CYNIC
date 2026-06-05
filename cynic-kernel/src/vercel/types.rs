//! Vercel API data structures.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VercelDeployment {
    pub uid: String,
    pub name: String,
    pub url: String,
    pub state: String,
    pub target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDeploymentRequest {
    pub name: String,
    pub project_id: String,
    pub git_source: GitSource,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GitSource {
    pub r#type: String, // "github" | "gitlab" | "bitbucket"
    pub r#ref: String,    // branch name
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct VercelError {
    pub code: String,
    pub message: String,
}
