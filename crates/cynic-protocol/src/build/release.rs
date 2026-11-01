use crate::RUNTIME_RELEASE_SIGNATURE_NAMESPACE;
use serde::{Deserialize, Serialize};

pub const RELEASE_PUBLICATION_DESIRED_STATE_SCHEMA: &str =
    "cynic.release_publication.desired_state.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasePublicationDesiredState {
    pub schema: String,
    pub storage_role: String,
    pub service_name: String,
    pub protocol: String,
    pub authentication_kind: String,
    pub credential_path: String,
    pub signing_key_path: String,
    pub signer_identity: String,
    pub signature_namespace: String,
}

pub fn validate_release_publication_desired_state(
    desired: &ReleasePublicationDesiredState,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != RELEASE_PUBLICATION_DESIRED_STATE_SCHEMA {
        blockers.push("release_publication_desired_state_schema_mismatch".to_string());
    }
    if desired.storage_role.trim().is_empty()
        || desired.service_name.trim().is_empty()
        || desired.signer_identity.trim().is_empty()
    {
        blockers.push("release_publication_desired_identity_missing".to_string());
    }
    if desired.protocol != "generic_https_put_get" || desired.authentication_kind != "bearer_file" {
        blockers.push("release_publication_desired_adapter_unsupported".to_string());
    }
    if desired.signature_namespace != RUNTIME_RELEASE_SIGNATURE_NAMESPACE {
        blockers.push("release_publication_desired_signature_namespace_mismatch".to_string());
    }
    if [&desired.credential_path, &desired.signing_key_path]
        .iter()
        .any(|path| !std::path::Path::new(path).is_absolute())
    {
        blockers.push("release_publication_desired_machine_path_not_absolute".to_string());
    }
    blockers
}
