use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use cynic_protocol::{SecurityEvidenceState};
use cynic_node_protocol::{NODE_ENROLLMENT_OBSERVATION_SCHEMA, NodeEnrollmentObservation, NodeEnrollmentRequest, SignedNodeEnrollmentGrant, node_enrollment_grant_signing_bytes, reconcile_node_enrollment, validate_signed_node_enrollment_grant};

#[derive(Debug, Clone)]
pub struct LocalNodeEnrollmentReader {
    request_path: PathBuf,
    grant_path: PathBuf,
    authority_public_key_path: PathBuf,
    workload_private_key_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NodeEnrollmentReadBack {
    pub state: SecurityEvidenceState,
    pub granted_capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
    pub grant_id: Option<String>,
    pub generation: Option<u64>,
    pub expires_at_unix_s: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct LocalEnrollmentBundle {
    schema: String,
    request: NodeEnrollmentRequest,
    grant: SignedNodeEnrollmentGrant,
}

impl LocalNodeEnrollmentReader {
    pub fn new(
        request_path: impl Into<PathBuf>,
        grant_path: impl Into<PathBuf>,
        authority_public_key_path: impl Into<PathBuf>,
        workload_private_key_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            request_path: request_path.into(),
            grant_path: grant_path.into(),
            authority_public_key_path: authority_public_key_path.into(),
            workload_private_key_path: workload_private_key_path.into(),
        }
    }

    pub fn read_back(&self, expected_node_id: &str) -> NodeEnrollmentReadBack {
        match self.try_read_back(expected_node_id) {
            Ok(read_back) => read_back,
            Err(error) => NodeEnrollmentReadBack {
                state: SecurityEvidenceState::Unknown,
                granted_capabilities: Vec::new(),
                evidence_refs: Vec::new(),
                unknowns: vec![error],
                grant_id: None,
                generation: None,
                expires_at_unix_s: None,
            },
        }
    }

    fn try_read_back(&self, expected_node_id: &str) -> Result<NodeEnrollmentReadBack, String> {
        let (request, signed) = self.read_pair()?;
        let violations = validate_signed_node_enrollment_grant(&signed);
        if !violations.is_empty() {
            return Err(violations.join(","));
        }
        if request.node_id != expected_node_id || signed.grant.node_id != expected_node_id {
            return Err("enrollment node identity does not match local configuration".to_string());
        }

        let authority_key = read_hex_32(&self.authority_public_key_path, "enrollment authority")?;
        let authority_key_id = format!("sha256:{}", hex::encode(Sha256::digest(authority_key)));
        if signed.authority_key_id != authority_key_id {
            return Err(
                "enrollment authority key does not match the configured trust root".to_string(),
            );
        }
        let verifying_key = VerifyingKey::from_bytes(&authority_key)
            .map_err(|_| "enrollment authority public key is invalid".to_string())?;
        let signature_bytes: [u8; 64] = hex::decode(&signed.signature_hex)
            .map_err(|_| "enrollment signature is not hex".to_string())?
            .try_into()
            .map_err(|_| "enrollment signature length is invalid".to_string())?;
        verifying_key
            .verify(
                &node_enrollment_grant_signing_bytes(&signed)
                    .map_err(|error| format!("encode enrollment grant: {error}"))?,
                &Signature::from_bytes(&signature_bytes),
            )
            .map_err(|_| "enrollment authority signature is invalid".to_string())?;

        let workload_secret = read_hex_32(&self.workload_private_key_path, "workload identity")?;
        let workload_public = ed25519_dalek::SigningKey::from_bytes(&workload_secret)
            .verifying_key()
            .to_bytes();
        let machine_identity_digest =
            format!("sha256:{}", hex::encode(Sha256::digest(workload_public)));
        let observation = NodeEnrollmentObservation {
            schema: NODE_ENROLLMENT_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_s: chrono::Utc::now().timestamp().max(0) as u64,
            node_id: expected_node_id.to_string(),
            machine_identity_digest,
            machine_identity: SecurityEvidenceState::Proven,
            authority_signature: SecurityEvidenceState::Proven,
            prior_enrollment: SecurityEvidenceState::Proven,
            evidence_refs: vec![authority_key_id, signed.grant.grant_id.clone()],
            unknowns: Vec::new(),
        };
        let plan = reconcile_node_enrollment(&request, &signed.grant, &observation);
        if plan.status != "converged" {
            return Err(format!(
                "enrollment read-back is not converged:{}",
                plan.blockers
                    .into_iter()
                    .chain(plan.unknowns)
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        Ok(NodeEnrollmentReadBack {
            state: SecurityEvidenceState::Proven,
            granted_capabilities: plan.granted_capabilities,
            evidence_refs: plan.evidence_refs,
            unknowns: Vec::new(),
            grant_id: Some(signed.grant.grant_id),
            generation: Some(signed.grant.generation),
            expires_at_unix_s: Some(signed.grant.expires_at_unix_s),
        })
    }

    fn read_pair(&self) -> Result<(NodeEnrollmentRequest, SignedNodeEnrollmentGrant), String> {
        if let Ok(bundle) =
            read_json::<LocalEnrollmentBundle>(&self.grant_path, "enrollment bundle")
        {
            if bundle.schema != "cynic.local_node_enrollment_bundle.v1" {
                return Err("local enrollment bundle schema is invalid".to_string());
            }
            return Ok((bundle.request, bundle.grant));
        }
        Ok((
            read_json(&self.request_path, "enrollment request")?,
            read_json(&self.grant_path, "enrollment grant")?,
        ))
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    let raw = std::fs::read(path).map_err(|error| format!("read {label}: {error}"))?;
    serde_json::from_slice(&raw).map_err(|error| format!("decode {label}: {error}"))
}

fn read_hex_32(path: &Path, label: &str) -> Result<[u8; 32], String> {
    let raw = std::fs::read_to_string(path).map_err(|error| format!("read {label}: {error}"))?;
    hex::decode(raw.trim())
        .map_err(|_| format!("{label} must be hex encoded"))?
        .try_into()
        .map_err(|_| format!("{label} must contain 32 bytes"))
}
