//! Control plan verifier for the CYNIC Trust-Chain vertical.
//!
//! Enforces that a NodeControlPlan is cryptographically signed by an authorized
//! Kernel root of trust, strictly monotonic in generation, and active in its validity window
//! before any host mutations are planned or applied.

use std::collections::HashMap;
use std::sync::Arc;

use cynic_node_protocol::node_control::{
    ControlPlanVerificationStatus, NodeDesiredState, SignedNodeControlPlan,
    verify_control_plan_preconditions,
};

use crate::crypto::{CryptoError, CryptoProvider, Ed25519Provider};

/// Cryptographic verifier for Kernel control plans.
pub struct PlanVerifier {
    crypto: Arc<dyn CryptoProvider>,
    authorized_kernel_keys: HashMap<String, Vec<u8>>,
}

impl Default for PlanVerifier {
    fn default() -> Self {
        Self {
            crypto: Arc::new(Ed25519Provider),
            authorized_kernel_keys: HashMap::new(),
        }
    }
}

impl PlanVerifier {
    pub fn new(crypto: Arc<dyn CryptoProvider>) -> Self {
        Self {
            crypto,
            authorized_kernel_keys: HashMap::new(),
        }
    }

    /// Registers a trusted Kernel authority public key.
    pub fn register_kernel_authority(
        &mut self,
        authority_id: &str,
        public_key: &[u8],
    ) -> Result<(), CryptoError> {
        if authority_id.trim().is_empty() {
            return Err(CryptoError::InvalidKey(
                "authority_id cannot be empty".into(),
            ));
        }
        if public_key.len() != 32 {
            return Err(CryptoError::InvalidKey(
                "kernel public key must be 32 bytes".into(),
            ));
        }
        self.authorized_kernel_keys
            .insert(authority_id.to_string(), public_key.to_vec());
        Ok(())
    }

    /// Signs a desired state to produce a SignedNodeControlPlan (used by Kernel / authority plane).
    pub fn sign_plan(
        &self,
        desired_state: NodeDesiredState,
        kernel_authority_id: &str,
        kernel_private_key: &[u8],
        ttl_s: u64,
        now_unix_s: u64,
    ) -> Result<SignedNodeControlPlan, CryptoError> {
        let expires_at_unix_s = now_unix_s.saturating_add(ttl_s);
        let dummy = SignedNodeControlPlan::new(
            desired_state.clone(),
            kernel_authority_id.to_string(),
            now_unix_s,
            expires_at_unix_s,
            String::new(),
        );

        let signing_bytes = dummy.signing_bytes();
        let signature = self.crypto.sign(&signing_bytes, kernel_private_key)?;
        let signature_hex = hex::encode(signature);

        let signed = SignedNodeControlPlan::new(
            desired_state,
            kernel_authority_id.to_string(),
            now_unix_s,
            expires_at_unix_s,
            signature_hex,
        );

        Ok(signed)
    }

    /// Verifies that a SignedNodeControlPlan is authorized, unexpired, monotonic, and authentically signed.
    pub fn verify_and_admit_plan(
        &self,
        signed_plan: &SignedNodeControlPlan,
        observed_generation: Option<u64>,
        now_unix_s: u64,
    ) -> Result<NodeDesiredState, ControlPlanVerificationStatus> {
        let authorized_ids: Vec<String> = self.authorized_kernel_keys.keys().cloned().collect();
        verify_control_plan_preconditions(
            signed_plan,
            observed_generation,
            &authorized_ids,
            now_unix_s,
        )?;

        let public_key = self
            .authorized_kernel_keys
            .get(&signed_plan.kernel_authority_id)
            .ok_or_else(|| {
                ControlPlanVerificationStatus::UntrustedAuthority(
                    signed_plan.kernel_authority_id.clone(),
                )
            })?;

        let signature = hex::decode(&signed_plan.signature_hex)
            .map_err(|_| ControlPlanVerificationStatus::InvalidSignature)?;

        let signing_bytes = signed_plan.signing_bytes();
        let valid = self
            .crypto
            .verify(&signing_bytes, &signature, public_key)
            .map_err(|_| ControlPlanVerificationStatus::InvalidSignature)?;

        if !valid {
            return Err(ControlPlanVerificationStatus::InvalidSignature);
        }

        Ok(signed_plan.desired_state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::RuntimeSeedRequest;
    use cynic_node_protocol::node_control::NODE_DESIRED_STATE_SCHEMA;

    fn fixture_desired(generation: u64) -> NodeDesiredState {
        NodeDesiredState {
            schema: NODE_DESIRED_STATE_SCHEMA.to_string(),
            node_id: "node-cortex".to_string(),
            role: "cortex".to_string(),
            generation,
            policy_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            release: RuntimeSeedRequest::example("node-cortex", "cynic-v0.1.0", "75b4cd4b"),
            required_capabilities: vec![],
            required_services: vec![],
            paused: false,
        }
    }

    #[test]
    fn valid_plan_is_verified_and_admitted() {
        let provider = Ed25519Provider;
        let mut verifier = PlanVerifier::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("keys");
        verifier
            .register_kernel_authority("kernel-prod", &kernel_public)
            .expect("register authority");

        let desired = fixture_desired(5);
        let now = 1700000000;
        let signed = verifier
            .sign_plan(desired.clone(), "kernel-prod", &kernel_secret, 3600, now)
            .expect("sign plan");

        let admitted = verifier
            .verify_and_admit_plan(&signed, Some(4), now + 10)
            .expect("admit plan");

        assert_eq!(admitted, desired);
    }

    #[test]
    fn tampered_plan_is_rejected() {
        let provider = Ed25519Provider;
        let mut verifier = PlanVerifier::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("keys");
        verifier
            .register_kernel_authority("kernel-prod", &kernel_public)
            .expect("register authority");

        let desired = fixture_desired(5);
        let now = 1700000000;
        let mut signed = verifier
            .sign_plan(desired, "kernel-prod", &kernel_secret, 3600, now)
            .expect("sign plan");

        // Tamper with generation after signing
        signed.desired_state.generation = 6;

        let result = verifier.verify_and_admit_plan(&signed, Some(4), now + 10);
        assert_eq!(result, Err(ControlPlanVerificationStatus::InvalidSignature));
    }

    #[test]
    fn stale_generation_is_rejected() {
        let provider = Ed25519Provider;
        let mut verifier = PlanVerifier::default();

        let (kernel_secret, kernel_public) = provider.generate_keypair().expect("keys");
        verifier
            .register_kernel_authority("kernel-prod", &kernel_public)
            .expect("register authority");

        let desired = fixture_desired(5);
        let now = 1700000000;
        let signed = verifier
            .sign_plan(desired, "kernel-prod", &kernel_secret, 3600, now)
            .expect("sign plan");

        // Observed generation is already 5
        let result = verifier.verify_and_admit_plan(&signed, Some(5), now + 10);
        assert_eq!(
            result,
            Err(ControlPlanVerificationStatus::StaleGeneration {
                observed: 5,
                plan: 5
            })
        );
    }
}
