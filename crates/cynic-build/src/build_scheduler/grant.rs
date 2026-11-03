//! Build workload grant issuance — pure logic over `BuildWorkloadGrantSignerPort`.
//!
//! Moved from `cynic-node/src/usecases/build_workload_grant_uc.rs` so the
//! scheduler can issue grants without depending on cynic-node.

use cynic_agent::DurableAgentWorkloadRecord;

use cynic_build::{
    BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE, SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA,
    SignedBuildWorkloadGrant, build_workload_grant_signing_bytes, derive_build_workload_grant,
    validate_signed_build_workload_grant,
};
use cynic_build::ports::BuildWorkloadGrantSignerPort;

/// Issue a signed build workload grant for a durable record.
pub fn issue_signed_build_workload_grant(
    durable: &DurableAgentWorkloadRecord,
    issuer_node_id: &str,
    now_unix_s: u64,
    signer: &dyn BuildWorkloadGrantSignerPort,
) -> Result<SignedBuildWorkloadGrant, Vec<String>> {
    let grant = derive_build_workload_grant(durable, issuer_node_id, now_unix_s)?;
    let mut signed = SignedBuildWorkloadGrant {
        schema: SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        authority_key_id: signer.authority_key_id().to_string(),
        signature_namespace: BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: String::new(),
    };
    let payload = build_workload_grant_signing_bytes(&signed)
        .map_err(|_| vec!["build_workload_grant_signing_payload_failed".to_string()])?;
    signed.signature_hex = signer
        .sign(&payload)
        .map_err(|_| vec!["build_workload_grant_machine_signing_failed".to_string()])?;
    let blockers = validate_signed_build_workload_grant(&signed);
    if blockers.is_empty() {
        Ok(signed)
    } else {
        Err(blockers)
    }
}
