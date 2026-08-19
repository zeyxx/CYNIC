use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameworkRole {
    Kernel,
    Node,
}

impl FrameworkRole {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "kernel" => Ok(Self::Kernel),
            "node" => Ok(Self::Node),
            _ => Err("requested role must be one of: kernel, node".to_string()),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Node => "node",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameworkEnrollmentRequestReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub request_id: String,
    pub seed_id: String,
    pub machine_id: String,
    pub requested_role: &'static str,
    pub identity_class: &'static str,
    pub admission_status: &'static str,
    pub required_proofs: Vec<&'static str>,
    pub blockers: Vec<&'static str>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Serialize)]
struct EnrollmentIntent<'a> {
    seed_id: &'a str,
    machine_id: &'a str,
    requested_role: &'a str,
}

pub fn build_framework_enrollment_request(
    seed_manifest_path: &str,
    machine_id: &str,
    requested_role: FrameworkRole,
) -> Result<FrameworkEnrollmentRequestReport, String> {
    validate_machine_id(machine_id)?;
    let seed_manifest = fs::read(seed_manifest_path)
        .map_err(|error| format!("failed to read seed manifest {seed_manifest_path}: {error}"))?;
    let seed: serde_json::Value = serde_json::from_slice(&seed_manifest)
        .map_err(|error| format!("failed to parse seed manifest {seed_manifest_path}: {error}"))?;
    validate_seed_manifest(&seed)?;
    let seed_id = seed["seed_id"]
        .as_str()
        .expect("validated seed_id")
        .to_string();
    let intent = EnrollmentIntent {
        seed_id: &seed_id,
        machine_id,
        requested_role: requested_role.as_str(),
    };
    let identity = serde_json::to_vec(&intent)
        .map_err(|error| format!("failed to serialize enrollment intent: {error}"))?;

    Ok(FrameworkEnrollmentRequestReport {
        schema: "cynic.framework_enrollment.request.v1",
        status: "pending_external_admission",
        mutates: false,
        request_id: format!("sha256:{}", hex::encode(Sha256::digest(identity))),
        seed_id,
        machine_id: machine_id.to_string(),
        requested_role: requested_role.as_str(),
        identity_class: "MachineEnrollment",
        admission_status: "external_authority_required",
        required_proofs: vec![
            "seed_artifacts_integrity_verified",
            "machine_identity_observed",
            "external_admission_grant_bound_to_request_id",
        ],
        blockers: vec!["external_enrollment_authority_unavailable"],
        rollback_intent: "discard this request; do not activate a role or mutate the target",
    })
}

fn validate_seed_manifest(seed: &serde_json::Value) -> Result<(), String> {
    if seed["schema"].as_str() != Some("cynic.framework_seed.v2") {
        return Err("seed manifest schema must be cynic.framework_seed.v2".to_string());
    }
    if seed["status"].as_str() != Some("seed_only") {
        return Err("seed manifest must have seed_only status before enrollment".to_string());
    }
    let seed_id = seed["seed_id"]
        .as_str()
        .ok_or_else(|| "seed manifest seed_id is missing".to_string())?;
    if !valid_sha256_id(seed_id) {
        return Err("seed manifest seed_id must be a sha256 digest".to_string());
    }
    if seed["artifacts"].as_array().is_none_or(Vec::is_empty) {
        return Err("seed manifest must declare at least one artifact".to_string());
    }
    if seed["enrollment"]["required"].as_bool() != Some(true)
        || seed["enrollment"]["identity_class"].as_str() != Some("MachineEnrollment")
    {
        return Err("seed manifest does not require MachineEnrollment".to_string());
    }
    Ok(())
}

fn valid_sha256_id(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn validate_machine_id(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(
            "machine id must be 1..128 ASCII alphanumeric characters plus . - _ or :".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_manifest() -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().expect("temporary seed manifest");
        std::fs::write(
            file.path(),
            r#"{
              "schema":"cynic.framework_seed.v2",
              "seed_id":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
              "status":"seed_only",
              "artifacts":[{"path":"install.sh"}],
              "enrollment":{"required":true,"identity_class":"MachineEnrollment"}
            }"#,
        )
        .expect("write seed manifest");
        file
    }

    #[test]
    fn enrollment_request_is_bound_to_seed_machine_and_role_without_mutation() {
        let seed = seed_manifest();
        let report = build_framework_enrollment_request(
            seed.path().to_str().expect("path"),
            "pve-vm-103",
            FrameworkRole::Kernel,
        )
        .expect("request");

        assert_eq!(report.status, "pending_external_admission");
        assert!(!report.mutates);
        assert_eq!(report.requested_role, "kernel");
        assert_eq!(report.admission_status, "external_authority_required");
        assert!(
            report
                .blockers
                .contains(&"external_enrollment_authority_unavailable")
        );
    }

    #[test]
    fn enrollment_request_rejects_invalid_machine_identifier() {
        let seed = seed_manifest();
        let error = build_framework_enrollment_request(
            seed.path().to_str().expect("path"),
            "pve; shutdown -h now",
            FrameworkRole::Node,
        )
        .expect_err("invalid machine id must fail");

        assert!(error.contains("machine id"));
    }

    #[test]
    fn enrollment_request_refuses_non_seed_manifest() {
        let seed = tempfile::NamedTempFile::new().expect("temporary file");
        std::fs::write(seed.path(), "{}").expect("write invalid manifest");
        let error = build_framework_enrollment_request(
            seed.path().to_str().expect("path"),
            "pve-vm-103",
            FrameworkRole::Node,
        )
        .expect_err("invalid manifest must fail");

        assert!(error.contains("schema"));
    }
}
