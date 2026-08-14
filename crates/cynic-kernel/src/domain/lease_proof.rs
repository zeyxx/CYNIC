use cynic_protocol::RuntimeProofBundle;

const MAX_PROOF_AGE_MS: u128 = 15 * 60 * 1_000;
const FUTURE_TOLERANCE_MS: u128 = 60 * 1_000;
const MAX_NORMALIZED_BUNDLE_BYTES: usize = 64 * 1_024;

#[derive(Debug, Clone)]
pub struct ValidatedLeaseProof {
    proof_ref: String,
    normalized_bundle_json: String,
}

impl ValidatedLeaseProof {
    pub fn validate(
        bundle: &RuntimeProofBundle,
        target: &str,
        now_unix_ms: u128,
    ) -> Result<Self, String> {
        bundle.validate_for_subject(target, now_unix_ms, MAX_PROOF_AGE_MS, FUTURE_TOLERANCE_MS)?;
        let normalized_bundle_json = bundle
            .normalized_json()
            .map_err(|error| format!("runtime proof bundle serialization failed: {error}"))?;
        if normalized_bundle_json.len() > MAX_NORMALIZED_BUNDLE_BYTES {
            return Err("runtime proof bundle exceeds 64 KiB".into());
        }
        let proof_ref = format!("blake3:{}", blake3::hash(normalized_bundle_json.as_bytes()));
        Ok(Self {
            proof_ref,
            normalized_bundle_json,
        })
    }

    pub fn proof_ref(&self) -> &str {
        &self.proof_ref
    }

    pub fn normalized_bundle_json(&self) -> &str {
        &self.normalized_bundle_json
    }
}
