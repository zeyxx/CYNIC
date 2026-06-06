//! Vercel API client implementation.

use super::types::{CreateDeploymentRequest, VercelDeployment, VercelError};
use reqwest::Client;

const API_BASE: &str = "https://api.vercel.com";

#[derive(Debug, Clone)]
pub struct VercelClient {
    client: Client,
    token: String,
}

impl VercelClient {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            token,
        }
    }

    pub async fn list_deployments(
        &self,
        app_name: &str,
    ) -> Result<Vec<VercelDeployment>, VercelError> {
        let url = format!("{}/v6/deployments?app={}", API_BASE, app_name);
        let res = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| VercelError {
                code: "network_error".to_string(),
                message: e.to_string(),
            })?;

        if !res.status().is_success() {
            let error: VercelError = res.json().await.map_err(|e| VercelError {
                code: "deserialization_error".to_string(),
                message: e.to_string(),
            })?;
            return Err(error);
        }

        #[derive(Debug, serde::Deserialize)]
        struct DeploymentsResponse {
            deployments: Vec<VercelDeployment>,
        }

        let body: DeploymentsResponse = res.json().await.map_err(|e| VercelError {
            code: "deserialization_error".to_string(),
            message: e.to_string(),
        })?;

        Ok(body.deployments)
    }

    pub async fn create_deployment(
        &self,
        request: CreateDeploymentRequest,
    ) -> Result<VercelDeployment, VercelError> {
        let url = format!("{}/v13/deployments", API_BASE);
        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&request)
            .send()
            .await
            .map_err(|e| VercelError {
                code: "network_error".to_string(),
                message: e.to_string(),
            })?;

        if !res.status().is_success() {
            let error: VercelError = res.json().await.map_err(|e| VercelError {
                code: "deserialization_error".to_string(),
                message: e.to_string(),
            })?;
            return Err(error);
        }

        let deployment: VercelDeployment = res.json().await.map_err(|e| VercelError {
            code: "deserialization_error".to_string(),
            message: e.to_string(),
        })?;

        Ok(deployment)
    }
}
