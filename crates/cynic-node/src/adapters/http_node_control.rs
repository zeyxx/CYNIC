use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signer, SigningKey};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ports::{
    agent_workload_delivery_port::AgentWorkloadDeliveryPort,
    node_control_plane_port::{NodeDesiredStatePullPort, NodeStatusPublishPort},
};
use cynic_protocol::{
    AgentWorkloadCompletionPublication, AgentWorkloadDelivery, AgentWorkloadDeliveryAcceptance,
    AgentWorkloadDeliveryStatus, NODE_ENROLLMENT_RENEWAL_REQUEST_SCHEMA,
    NODE_ENROLLMENT_REQUEST_SCHEMA, NodeControlPlan, NodeDesiredState,
    NodeEnrollmentRenewalRequest, NodeEnrollmentRequest, SignedNodeEnrollmentGrant,
    validate_agent_workload_delivery, validate_node_control_plan, validate_node_desired_state,
};

#[derive(Debug, Clone)]
pub struct HttpNodeControlAdapter {
    client: Client,
    base_url: String,
    identity: FileWorkloadIdentity,
}

#[derive(Debug, Clone)]
struct FileWorkloadIdentity {
    signing_key_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct DesiredStateResponse {
    schema: String,
    status: String,
    value: NodeDesiredState,
}

#[derive(Debug, Deserialize)]
struct NodeStatusResponse {
    schema: String,
    status: String,
    value: NodeControlPlan,
}

#[derive(Debug, Deserialize)]
struct EnrollmentResponse {
    schema: String,
    status: String,
    grant: SignedNodeEnrollmentGrant,
}

#[derive(Debug, Deserialize)]
struct WorkloadDeliveryResponse<T> {
    schema: String,
    status: String,
    value: T,
}

#[derive(Debug, Serialize)]
struct WorkloadDeliveryAcceptRequest<'a> {
    consumer_instance_id: &'a str,
    record_digest: &'a str,
    lease_ttl_ms: u64,
    prior_lease_id: Option<&'a str>,
    prior_fencing_token: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LocalEnrollmentBundle {
    schema: String,
    request: NodeEnrollmentRequest,
    grant: SignedNodeEnrollmentGrant,
}

impl HttpNodeControlAdapter {
    pub fn new(
        base_url: impl Into<String>,
        signing_key_path: impl Into<PathBuf>,
        timeout: Duration,
    ) -> Result<Self, String> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if !matches!(base_url.as_str(), value if value.starts_with("https://") || value.starts_with("http://127.0.0.1:") || value.starts_with("http://[::1]:"))
        {
            return Err("node control requires TLS or a loopback transport".to_string());
        }
        let signing_key_path = signing_key_path.into();
        validate_private_key_file(&signing_key_path)?;
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|error| format!("build node-control HTTP client: {error}"))?;
        Ok(Self {
            client,
            base_url,
            identity: FileWorkloadIdentity { signing_key_path },
        })
    }

    async fn execute(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
    ) -> Result<reqwest::Response, String> {
        self.execute_with_token(method, path, body, None).await
    }

    async fn execute_with_token(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
        enrollment_token: Option<&str>,
    ) -> Result<reqwest::Response, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "system clock precedes unix epoch".to_string())?
            .as_secs()
            .to_string();
        let signing_key = self.identity.load()?;
        let public_key = hex::encode(signing_key.verifying_key().as_bytes());
        let payload = signed_payload(&timestamp, method.as_str(), path, &body);
        let signature = hex::encode(signing_key.sign(&payload).to_bytes());
        let mut request = self
            .client
            .request(method, format!("{}{}", self.base_url, path))
            .header("X-Cynic-Public-Key", public_key)
            .header("X-Cynic-Signature", signature)
            .header("X-Cynic-Timestamp", timestamp)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        if let Some(token) = enrollment_token {
            request = request.header("X-Cynic-Enrollment-Token", token);
        }
        request
            .body(body)
            .send()
            .await
            .map_err(|error| format!("node-control request failed: {error}"))
    }

    pub async fn bootstrap_enrollment(
        &self,
        params: BootstrapEnrollmentParams<'_>,
    ) -> Result<(), String> {
        let signing_key = self.identity.load()?;
        let machine_public_key_hex = hex::encode(signing_key.verifying_key().as_bytes());
        let machine_identity_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
        );
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "system clock precedes unix epoch".to_string())?
            .as_secs();
        let expires_at_unix_s = now
            .checked_add(params.request_ttl_secs)
            .ok_or_else(|| "enrollment request expiry overflow".to_string())?;
        let nonce_material = format!("{}:{now}:{machine_identity_digest}", params.node_id);
        let nonce = hex::encode(Sha256::digest(nonce_material.as_bytes()));
        let request = NodeEnrollmentRequest {
            schema: NODE_ENROLLMENT_REQUEST_SCHEMA.to_string(),
            request_id: format!("enroll:{}", &nonce[..32]),
            seed_id: params.seed_id.to_string(),
            node_id: params.node_id.to_string(),
            role: params.role.to_string(),
            machine_identity_digest,
            machine_public_key_hex,
            nonce,
            issued_at_unix_s: now,
            expires_at_unix_s,
            requested_capabilities: params.requested_capabilities,
        };
        persist_json_atomic(params.request_path, &request)?;
        let token = std::fs::read_to_string(params.token_path)
            .map_err(|error| format!("read machine enrollment token: {error}"))?;
        let body = serde_json::to_vec(&request)
            .map_err(|error| format!("encode enrollment request: {error}"))?;
        let response = self
            .execute_with_token(Method::POST, "/node-enrollment", body, Some(token.trim()))
            .await?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read enrollment response: {error}"))?;
        if !status.is_success() {
            return Err(format!("enrollment endpoint returned {status}"));
        }
        let envelope: EnrollmentResponse = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode enrollment response: {error}"))?;
        if envelope.schema != "cynic.node_enrollment_response.v1"
            || envelope.status != "issued_and_read_back"
            || envelope.grant.grant.node_id != params.node_id
            || envelope.grant.grant.request_id != request.request_id
        {
            return Err("enrollment response binding is invalid".to_string());
        }
        persist_json_atomic(
            params.grant_path,
            &LocalEnrollmentBundle {
                schema: "cynic.local_node_enrollment_bundle.v1".to_string(),
                request,
                grant: envelope.grant,
            },
        )
    }

    pub async fn renew_enrollment(&self, params: RenewEnrollmentParams<'_>) -> Result<(), String> {
        let signing_key = self.identity.load()?;
        let machine_public_key_hex = hex::encode(signing_key.verifying_key().as_bytes());
        let machine_identity_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(signing_key.verifying_key().as_bytes()))
        );
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "system clock precedes unix epoch".to_string())?
            .as_secs();
        let expires_at_unix_s = now
            .checked_add(params.request_ttl_secs)
            .ok_or_else(|| "renewal request expiry overflow".to_string())?;
        let nonce = hex::encode(Sha256::digest(
            format!(
                "renew:{}:{}:{now}:{machine_identity_digest}",
                params.node_id, params.expected_generation
            )
            .as_bytes(),
        ));
        let request = NodeEnrollmentRequest {
            schema: NODE_ENROLLMENT_REQUEST_SCHEMA.to_string(),
            request_id: format!("renew:{}", &nonce[..32]),
            seed_id: params.seed_id.to_string(),
            node_id: params.node_id.to_string(),
            role: params.role.to_string(),
            machine_identity_digest,
            machine_public_key_hex,
            nonce,
            issued_at_unix_s: now,
            expires_at_unix_s,
            requested_capabilities: params.requested_capabilities,
        };
        let renewal = NodeEnrollmentRenewalRequest {
            schema: NODE_ENROLLMENT_RENEWAL_REQUEST_SCHEMA.to_string(),
            current_grant_id: params.current_grant_id.to_string(),
            expected_generation: params.expected_generation,
            request: request.clone(),
        };
        let body = serde_json::to_vec(&renewal)
            .map_err(|error| format!("encode enrollment renewal: {error}"))?;
        let response = self
            .execute(Method::POST, "/node-enrollment/renew", body)
            .await?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("read renewal response: {error}"))?;
        if !status.is_success() {
            return Err(format!("enrollment renewal returned {status}"));
        }
        let envelope: EnrollmentResponse = serde_json::from_slice(&bytes)
            .map_err(|error| format!("decode renewal response: {error}"))?;
        if envelope.schema != "cynic.node_enrollment_response.v1"
            || envelope.status != "issued_and_read_back"
            || envelope.grant.grant.generation != params.expected_generation.saturating_add(1)
            || envelope.grant.grant.request_id != request.request_id
        {
            return Err("renewal response fencing or binding is invalid".to_string());
        }
        persist_json_atomic(
            params.bundle_path,
            &LocalEnrollmentBundle {
                schema: "cynic.local_node_enrollment_bundle.v1".to_string(),
                request,
                grant: envelope.grant,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct BootstrapEnrollmentParams<'a> {
    pub node_id: &'a str,
    pub role: &'a str,
    pub seed_id: &'a str,
    pub requested_capabilities: Vec<String>,
    pub request_ttl_secs: u64,
    pub token_path: &'a Path,
    pub request_path: &'a Path,
    pub grant_path: &'a Path,
}

#[derive(Debug, Clone)]
pub struct RenewEnrollmentParams<'a> {
    pub node_id: &'a str,
    pub role: &'a str,
    pub seed_id: &'a str,
    pub requested_capabilities: Vec<String>,
    pub request_ttl_secs: u64,
    pub current_grant_id: &'a str,
    pub expected_generation: u64,
    pub bundle_path: &'a Path,
}

fn persist_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "enrollment path has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("create enrollment state directory: {error}"))?;
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("encode enrollment state: {error}"))?;
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("write enrollment state: {error}"))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("activate enrollment state: {error}"))?;
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync enrollment state: {error}"))
}

impl NodeDesiredStatePullPort for HttpNodeControlAdapter {
    async fn pull_desired_state(&self, node_id: &str) -> Result<Option<NodeDesiredState>, String> {
        let path = format!("/node-control/{node_id}/desired-state");
        let response = self.execute(Method::GET, &path, Vec::new()).await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| format!("read desired-state response: {error}"))?;
        if !status.is_success() {
            return Err(format!("desired-state endpoint returned {status}"));
        }
        let envelope: DesiredStateResponse = serde_json::from_slice(&body)
            .map_err(|error| format!("decode desired-state response: {error}"))?;
        if envelope.schema != "cynic.node_desired_state_response.v1"
            || envelope.status != "observed"
        {
            return Err("desired-state response envelope is invalid".to_string());
        }
        let violations = validate_node_desired_state(&envelope.value);
        if !violations.is_empty() {
            return Err(format!(
                "desired state is invalid: {}",
                violations.join(",")
            ));
        }
        Ok(Some(envelope.value))
    }
}

impl NodeStatusPublishPort for HttpNodeControlAdapter {
    async fn publish_node_status(&self, plan: &NodeControlPlan) -> Result<String, String> {
        let violations = validate_node_control_plan(plan);
        if !violations.is_empty() {
            return Err(format!("node status is invalid: {}", violations.join(",")));
        }
        let path = format!("/node-control/{}/status", plan.node_id);
        let body =
            serde_json::to_vec(plan).map_err(|error| format!("encode node status: {error}"))?;
        let response = self.execute(Method::POST, &path, body).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("status endpoint returned {status}"));
        }
        let read_back = self.execute(Method::GET, &path, Vec::new()).await?;
        let read_back_status = read_back.status();
        let read_back_body = read_back
            .bytes()
            .await
            .map_err(|error| format!("read status read-back response: {error}"))?;
        if !read_back_status.is_success() {
            return Err(format!("status read-back returned {read_back_status}"));
        }
        let envelope: NodeStatusResponse = serde_json::from_slice(&read_back_body)
            .map_err(|error| format!("decode status read-back: {error}"))?;
        if envelope.schema != "cynic.node_status_response.v1"
            || envelope.status != "observed"
            || envelope.value != *plan
        {
            return Err("status read-back does not match the published plan".to_string());
        }
        Ok(format!(
            "sha256:{}",
            hex::encode(Sha256::digest(&read_back_body))
        ))
    }
}

impl AgentWorkloadDeliveryPort for HttpNodeControlAdapter {
    async fn pending_deliveries(
        &self,
        node_id: &str,
    ) -> Result<Vec<AgentWorkloadDelivery>, String> {
        let path = format!("/node-control/{node_id}/agent-workloads");
        let response = self.execute(Method::GET, &path, Vec::new()).await?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| format!("read workload delivery response: {error}"))?;
        if !status.is_success() {
            return Err(format!("workload delivery endpoint returned {status}"));
        }
        let envelope: WorkloadDeliveryResponse<Vec<AgentWorkloadDelivery>> =
            serde_json::from_slice(&body)
                .map_err(|error| format!("decode workload deliveries: {error}"))?;
        if envelope.schema != "cynic.agent_workload_delivery_list_response.v1"
            || envelope.status != "observed"
        {
            return Err("workload delivery list envelope is invalid".to_string());
        }
        for delivery in &envelope.value {
            let violations = validate_agent_workload_delivery(delivery);
            if !violations.is_empty() || delivery.target_node_id != node_id {
                return Err(format!(
                    "workload delivery is invalid: {}",
                    violations.join(",")
                ));
            }
        }
        Ok(envelope.value)
    }

    async fn accept_delivery(
        &self,
        node_id: &str,
        workload_id: &str,
        acceptance: &AgentWorkloadDeliveryAcceptance,
    ) -> Result<AgentWorkloadDelivery, String> {
        let path = format!("/node-control/{node_id}/agent-workloads/{workload_id}/accept");
        let body = serde_json::to_vec(&WorkloadDeliveryAcceptRequest {
            consumer_instance_id: &acceptance.consumer_instance_id,
            record_digest: &acceptance.record_digest,
            lease_ttl_ms: acceptance.lease_ttl_ms,
            prior_lease_id: acceptance.prior_lease_id.as_deref(),
            prior_fencing_token: acceptance.prior_fencing_token,
        })
        .map_err(|error| format!("encode workload acceptance: {error}"))?;
        let response = self.execute(Method::POST, &path, body).await?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| format!("read workload acceptance response: {error}"))?;
        if !status.is_success() {
            return Err(format!("workload acceptance endpoint returned {status}"));
        }
        let envelope: WorkloadDeliveryResponse<AgentWorkloadDelivery> =
            serde_json::from_slice(&body)
                .map_err(|error| format!("decode workload acceptance: {error}"))?;
        if envelope.schema != "cynic.agent_workload_delivery_response.v1"
            || envelope.status != "accepted"
            || envelope.value.envelope.workload_id != workload_id
            || envelope.value.accepted_record_digest.as_deref()
                != Some(acceptance.record_digest.as_str())
            || envelope
                .value
                .acceptance_lease
                .as_ref()
                .is_none_or(|lease| lease.consumer_instance_id != acceptance.consumer_instance_id)
        {
            return Err("workload acceptance read-back is invalid".to_string());
        }
        Ok(envelope.value)
    }

    async fn publish_completion(
        &self,
        publication: &AgentWorkloadCompletionPublication,
    ) -> Result<AgentWorkloadDelivery, String> {
        let path = format!(
            "/node-control/{}/agent-workloads/{}/completion",
            publication.node_id, publication.workload_id
        );
        let body = serde_json::to_vec(publication)
            .map_err(|error| format!("encode workload completion publication: {error}"))?;
        let response = self.execute(Method::POST, &path, body).await?;
        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|error| format!("read workload completion response: {error}"))?;
        if !status.is_success() {
            return Err(format!("workload completion endpoint returned {status}"));
        }
        let envelope: WorkloadDeliveryResponse<AgentWorkloadDelivery> =
            serde_json::from_slice(&body)
                .map_err(|error| format!("decode workload completion: {error}"))?;
        if envelope.schema != "cynic.agent_workload_delivery_response.v1"
            || envelope.status != "terminal"
            || envelope.value.status != AgentWorkloadDeliveryStatus::Terminal
            || envelope.value.envelope.workload_id != publication.workload_id
            || envelope.value.terminal_record_digest.as_deref()
                != Some(publication.durable_record.record_digest.as_str())
            || !validate_agent_workload_delivery(&envelope.value).is_empty()
        {
            return Err("workload completion read-back is invalid".to_string());
        }
        Ok(envelope.value)
    }
}

impl FileWorkloadIdentity {
    fn load(&self) -> Result<SigningKey, String> {
        let raw = std::fs::read_to_string(&self.signing_key_path).map_err(|error| {
            format!(
                "read workload identity {}: {error}",
                self.signing_key_path.display()
            )
        })?;
        let bytes = hex::decode(raw.trim())
            .map_err(|_| "workload signing key must be hex encoded".to_string())?;
        let secret: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "workload signing key must contain 32 bytes".to_string())?;
        Ok(SigningKey::from_bytes(&secret))
    }
}

fn signed_payload(timestamp: &str, method: &str, path: &str, body: &[u8]) -> Vec<u8> {
    let mut payload = timestamp.as_bytes().to_vec();
    payload.extend_from_slice(method.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(path.as_bytes());
    payload.push(b'|');
    payload.extend_from_slice(body);
    payload
}

fn validate_private_key_file(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || !path.is_file() {
        return Err("workload signing key must be an existing absolute file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mode = std::fs::metadata(path)
            .map_err(|error| format!("inspect workload identity: {error}"))?
            .mode()
            & 0o777;
        if mode & 0o027 != 0 {
            return Err(
                "workload signing key must not be group-writable or accessible by others"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_payload_matches_kernel_wire_format() {
        assert_eq!(
            signed_payload("12", "GET", "/node-control/node-a/desired-state", &[]),
            b"12GET|/node-control/node-a/desired-state|"
        );
    }
}
