//! Vercel MCP tools.

use rmcp::{
    ErrorData as McpError, handler::server::wrapper::Parameters, model::CallToolResult, tool,
    tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::vercel::{
    client::VercelClient,
    types::{CreateDeploymentRequest, GitSource},
};

use super::CynicMcp;

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct ListDeployParams {
    pub app_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub(crate) struct DeployParams {
    pub name: String,
    pub project_id: String,
    pub git_source: GitSource,
}

#[tool_router(router = tool_router_vercel, vis = "pub(super)")]
impl CynicMcp {
    #[tool(
        name = "cynic_vercel_list_deployments",
        description = "List Vercel deployments for an app. Requires VERCEL_API_TOKEN environment variable."
    )]
    pub(crate) async fn cynic_vercel_list_deployments(
        &self,
        params: Parameters<ListDeployParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = std::env::var("VERCEL_API_TOKEN")
            .map_err(|_| McpError::internal_error("VERCEL_API_TOKEN not set", None))?;
        let client = VercelClient::new(token);

        match client.list_deployments(&params.0.app_name).await {
            Ok(deployments) => {
                let result =
                    serde_json::to_string(&deployments).unwrap_or_else(|_| "[]".to_string());
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    result,
                )]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("Vercel API error: {:?}", e),
                None,
            )),
        }
    }

    #[tool(
        name = "cynic_vercel_create_deployment",
        description = "Create a new Vercel deployment. Requires VERCEL_API_TOKEN environment variable."
    )]
    pub(crate) async fn cynic_vercel_create_deployment(
        &self,
        params: Parameters<DeployParams>,
    ) -> Result<CallToolResult, McpError> {
        let token = std::env::var("VERCEL_API_TOKEN")
            .map_err(|_| McpError::internal_error("VERCEL_API_TOKEN not set", None))?;
        let client = VercelClient::new(token);

        let request = CreateDeploymentRequest {
            name: params.0.name,
            project_id: params.0.project_id,
            git_source: params.0.git_source,
        };

        match client.create_deployment(request).await {
            Ok(deployment) => {
                let result =
                    serde_json::to_string(&deployment).unwrap_or_else(|_| "{}".to_string());
                Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    result,
                )]))
            }
            Err(e) => Err(McpError::internal_error(
                format!("Vercel API error: {:?}", e),
                None,
            )),
        }
    }
}
