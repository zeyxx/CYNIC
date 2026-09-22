use cynic_protocol::LeaseGuard;
use cynic_workload::{
    AgentWorkspaceClaimAdmission, AgentWorkspaceClaimIntent, AgentWorkspaceClaimObservation,
    AgentWorkspaceClaimOutcome,
};
use serde::de::DeserializeOwned;

use crate::ports::agent_workspace_port::{AgentWorkspaceClaimStorePort, AgentWorkspacePortError};

#[derive(Debug, Clone)]
pub struct KernelAgentWorkspaceClaimStore {
    client: reqwest::Client,
    endpoint: String,
    api_key: Option<String>,
    agent_id: String,
}

impl KernelAgentWorkspaceClaimStore {
    pub fn new(
        endpoint: impl Into<String>,
        api_key: Option<String>,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            api_key,
            agent_id: agent_id.into(),
        }
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, AgentWorkspacePortError> {
        let mut request = self
            .client
            .post(format!("{}{}", self.endpoint, path))
            .json(&body);
        if let Some(api_key) = &self.api_key {
            request = request.bearer_auth(api_key);
        }
        let response = request.send().await.map_err(|failure| {
            error(
                "workspace_claim_kernel_unreachable",
                format!("kernel claim request failed: {failure}"),
            )
        })?;
        let status = response.status();
        let payload = response.text().await.map_err(|failure| {
            error(
                "workspace_claim_kernel_response_unreadable",
                format!("kernel claim response could not be read: {failure}"),
            )
        })?;
        if !status.is_success() {
            return Err(error(
                "workspace_claim_kernel_rejected",
                format!("kernel claim request returned {status}: {payload}"),
            ));
        }
        serde_json::from_str(&payload).map_err(|failure| {
            error(
                "workspace_claim_kernel_response_invalid",
                format!("kernel claim response is not valid contract JSON: {failure}"),
            )
        })
    }
}

impl AgentWorkspaceClaimStorePort for KernelAgentWorkspaceClaimStore {
    async fn reserve(
        &self,
        intent: &AgentWorkspaceClaimIntent,
        admission: &AgentWorkspaceClaimAdmission,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimOutcome, AgentWorkspacePortError> {
        self.post(
            "/agent-workspace/claims/reserve",
            serde_json::json!({
                "intent": intent,
                "admission": admission,
                "now_ms": now_ms
            }),
        )
        .await
    }

    async fn repository_claims(
        &self,
        repository_id: &str,
        now_ms: u64,
    ) -> Result<Vec<AgentWorkspaceClaimObservation>, AgentWorkspacePortError> {
        self.post(
            "/agent-workspace/claims/list",
            serde_json::json!({
                "agent_id": self.agent_id,
                "repository_id": repository_id,
                "now_ms": now_ms
            }),
        )
        .await
    }

    async fn renew(
        &self,
        guard: &LeaseGuard,
        requested_ttl_ms: u64,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimObservation, AgentWorkspacePortError> {
        self.post(
            "/agent-workspace/claims/renew",
            serde_json::json!({
                "guard": guard,
                "requested_ttl_ms": requested_ttl_ms,
                "now_ms": now_ms
            }),
        )
        .await
    }

    async fn release(
        &self,
        guard: &LeaseGuard,
        now_ms: u64,
    ) -> Result<(), AgentWorkspacePortError> {
        let _: serde_json::Value = self
            .post(
                "/agent-workspace/claims/release",
                serde_json::json!({"guard": guard, "now_ms": now_ms}),
            )
            .await?;
        Ok(())
    }

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        proof_ref: &str,
        now_ms: u64,
    ) -> Result<(), AgentWorkspacePortError> {
        let _: serde_json::Value = self
            .post(
                "/agent-workspace/claims/finalize",
                serde_json::json!({
                    "guard": guard,
                    "proof_ref": proof_ref,
                    "now_ms": now_ms
                }),
            )
            .await?;
        Ok(())
    }
}

fn error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}
