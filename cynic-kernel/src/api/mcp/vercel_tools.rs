//! Vercel MCP tools.

use std::sync::Arc;
use crate::{
    api::mcp::{error::McpError, types::{CallToolResult, Parameters}, AppState},
    vercel::client::VercelClient,
    vercel::types::{CreateDeploymentRequest, GitSource},
};
use serde::Deserialize;
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListDeployParams {
    pub app_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeployParams {
    pub name: String,
    pub project_id: String,
    pub git_source: GitSource,
}

pub async fn cynic_vercel_list_deployments(
    _state: Arc<AppState>,
    params: Parameters<ListDeployParams>,
) -> Result<CallToolResult, McpError> {
    let token = std::env::var("VERCEL_API_TOKEN").map_err(|_| McpError::internal("VERCEL_API_TOKEN not set"))?;
    let client = VercelClient::new(token);

    match client.list_deployments(&params.app_name).await {
        Ok(deployments) => {
            let result = serde_json::to_string(&deployments).unwrap_or_else(|_| "[]".to_string());
            Ok(CallToolResult::new(vec![rmcp::model::Content::text(result)]))
        }
        Err(e) => Err(McpError::internal(format!("Vercel API error: {:?}", e), None)),
    }
}

pub async fn cynic_vercel_create_deployment(
    _state: Arc<AppState>,
    params: Parameters<DeployParams>,
) -> Result<CallToolResult, McpError> {
    let token = std::env::var("VERCEL_API_TOKEN").map_err(|_| McpError::internal("VERCEL_API_TOKEN not set"))?;
    let client = VercelClient::new(token);
    
    let request = CreateDeploymentRequest {
        name: params.name,
        project_id: params.project_id,
        git_source: params.git_source,
    };

    match client.create_deployment(request).await {
        Ok(deployment) => {
            let result = serde_json::to_string(&deployment).unwrap_or_else(|_| "{}".to_string());
            Ok(CallToolResult::new(vec![rmcp::model::Content::text(result)]))
        }
        Err(e) => Err(McpError::internal(format!("Vercel API error: {:?}", e), None)),
    }
}
