use cynic_protocol::{
    LeaseAction, LeaseGrant, LeaseGuard, LeaseMutationOutcome, LeaseVerification,
    RuntimeProofBundle, coord_claim_path, coord_finalize_path, coord_release_path,
    coord_verify_path,
};
use serde::de::DeserializeOwned;

use crate::ports::source_repository_proof_commit_port::{
    SourceRepositoryProofCommitError, SourceRepositoryProofCommitPort,
};

#[derive(Debug, Clone)]
pub struct KernelSourceRepositoryProofCommit {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
}

impl KernelSourceRepositoryProofCommit {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, SourceRepositoryProofCommitError> {
        let response = self
            .client
            .post(format!("{}{}", self.endpoint, path))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|failure| {
                error(
                    "source_bootstrap_kernel_unreachable",
                    format!("kernel request failed: {failure}"),
                )
            })?;
        let status = response.status();
        let payload = response.text().await.map_err(|failure| {
            error(
                "source_bootstrap_kernel_response_unreadable",
                format!("kernel response could not be read: {failure}"),
            )
        })?;
        if !status.is_success() {
            return Err(error(
                "source_bootstrap_kernel_rejected",
                format!("kernel returned {status}: {payload}"),
            ));
        }
        serde_json::from_str(&payload).map_err(|failure| {
            error(
                "source_bootstrap_kernel_response_invalid",
                format!("kernel response is not valid contract JSON: {failure}"),
            )
        })
    }
}

#[derive(serde::Deserialize)]
struct ClaimResponse {
    status: String,
    lease: Option<LeaseGrant>,
}

#[derive(serde::Deserialize)]
struct MutationResponse {
    status: String,
    result: LeaseMutationOutcome,
}

impl SourceRepositoryProofCommitPort for KernelSourceRepositoryProofCommit {
    async fn claim(
        &self,
        agent_id: &str,
        target: &str,
    ) -> Result<LeaseGuard, SourceRepositoryProofCommitError> {
        let response: ClaimResponse = self
            .post(
                coord_claim_path(),
                serde_json::json!({
                    "agent_id": agent_id,
                    "target": target,
                    "claim_type": "source_repository_bootstrap"
                }),
            )
            .await?;
        let lease = response
            .lease
            .filter(|_| response.status == "claimed")
            .ok_or_else(|| {
                error(
                    "source_bootstrap_lease_not_claimed",
                    format!("kernel coordination status was '{}'", response.status),
                )
            })?;
        Ok(LeaseGuard {
            agent_id: agent_id.to_string(),
            target: target.to_string(),
            lease_id: lease.lease_id,
            fencing_token: lease.fencing_token,
        })
    }

    async fn verify(
        &self,
        guard: &LeaseGuard,
    ) -> Result<LeaseVerification, SourceRepositoryProofCommitError> {
        self.post(
            coord_verify_path(),
            serde_json::to_value(guard).map_err(|failure| {
                error(
                    "source_bootstrap_lease_request_invalid",
                    failure.to_string(),
                )
            })?,
        )
        .await
    }

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        bundle: &RuntimeProofBundle,
    ) -> Result<String, SourceRepositoryProofCommitError> {
        let response: MutationResponse = self
            .post(
                coord_finalize_path(),
                serde_json::json!({
                    "agent_id": guard.agent_id,
                    "target": guard.target,
                    "lease_id": guard.lease_id,
                    "fencing_token": guard.fencing_token,
                    "proof_bundle": bundle
                }),
            )
            .await?;
        match response.result {
            LeaseMutationOutcome::Applied {
                action: LeaseAction::Finalize,
                proof_ref: Some(proof_ref),
                ..
            } if response.status == "applied" => Ok(proof_ref),
            _ => Err(error(
                "source_bootstrap_proof_not_committed",
                "kernel did not return a finalized proof reference",
            )),
        }
    }

    async fn release(&self, guard: &LeaseGuard) -> Result<(), SourceRepositoryProofCommitError> {
        let response: MutationResponse = self
            .post(
                coord_release_path(),
                serde_json::to_value(guard).map_err(|failure| {
                    error(
                        "source_bootstrap_lease_request_invalid",
                        failure.to_string(),
                    )
                })?,
            )
            .await?;
        match response.result {
            LeaseMutationOutcome::Applied {
                action: LeaseAction::Release,
                ..
            } if response.status == "applied" => Ok(()),
            _ => Err(error(
                "source_bootstrap_lease_not_released",
                "kernel did not confirm lease release",
            )),
        }
    }
}

fn error(code: impl Into<String>, message: impl Into<String>) -> SourceRepositoryProofCommitError {
    SourceRepositoryProofCommitError::new(code, message)
}
