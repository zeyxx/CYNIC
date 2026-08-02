use serde::{Deserialize, Serialize};

pub const BUILD_AGENT_REQUEST_SCHEMA: &str = "cynic.build.agent_request.v1";
pub const BUILD_AGENT_REQUEST_ADMISSION_SCHEMA: &str = "cynic.build.agent_request_admission.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequest {
    pub schema: String,
    pub request_id: String,
    pub requested_by: String,
    pub source_commit: String,
    pub action: String,
    pub desired_role: String,
    pub mutation_authorized: bool,
    pub proof_requirements: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildAgentRequestAdmission {
    pub schema: String,
    pub status: String,
    pub request_id: String,
    pub source_commit: String,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub fn plan_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, false)
}

pub fn admit_build_agent_request(request: &BuildAgentRequest) -> BuildAgentRequestAdmission {
    assess(request, true)
}

fn assess(
    request: &BuildAgentRequest,
    require_mutation_authority: bool,
) -> BuildAgentRequestAdmission {
    let mut blockers = Vec::new();
    if request.schema != BUILD_AGENT_REQUEST_SCHEMA {
        blockers.push("build_agent_request_schema_mismatch".to_string());
    }
    if request.request_id.trim().is_empty() {
        blockers.push("build_agent_request_id_missing".to_string());
    }
    if request.requested_by.trim().is_empty() {
        blockers.push("build_agent_request_principal_missing".to_string());
    }
    if !is_commit_id(&request.source_commit) {
        blockers.push("build_agent_request_source_commit_invalid".to_string());
    }
    if !matches!(
        request.action.as_str(),
        "make:check" | "make:build" | "make:build-release"
    ) {
        blockers.push("build_agent_request_action_not_allowed".to_string());
    }
    if request.desired_role != "build_runner" {
        blockers.push("build_agent_request_role_mismatch".to_string());
    }
    if require_mutation_authority && !request.mutation_authorized {
        blockers.push("build_agent_request_dispatch_not_authorized".to_string());
    }
    if request.proof_requirements.is_empty() {
        blockers.push("build_agent_request_proof_requirements_missing".to_string());
    }
    if request.evidence_refs.is_empty() {
        blockers.push("build_agent_request_source_evidence_missing".to_string());
    }
    BuildAgentRequestAdmission {
        schema: BUILD_AGENT_REQUEST_ADMISSION_SCHEMA.to_string(),
        status: if blockers.is_empty() {
            if require_mutation_authority {
                "admitted"
            } else {
                "planned"
            }
        } else {
            "blocked"
        }
        .to_string(),
        request_id: request.request_id.clone(),
        source_commit: request.source_commit.clone(),
        blockers,
        evidence_refs: request.evidence_refs.clone(),
    }
}

fn is_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(mutation_authorized: bool) -> BuildAgentRequest {
        BuildAgentRequest {
            schema: BUILD_AGENT_REQUEST_SCHEMA.to_string(),
            request_id: "validate-abc".to_string(),
            requested_by: "codex-session-1".to_string(),
            source_commit: "a".repeat(40),
            action: "make:check".to_string(),
            desired_role: "build_runner".to_string(),
            mutation_authorized,
            proof_requirements: vec![
                "canonical_gate_report".to_string(),
                "source_commit_read_back".to_string(),
            ],
            evidence_refs: vec!["forgejo:refs/heads/change@aaaaaaaa".to_string()],
        }
    }

    #[test]
    fn planning_does_not_require_dispatch_authority() {
        assert_eq!(plan_build_agent_request(&request(false)).status, "planned");
    }

    #[test]
    fn dispatch_requires_explicit_authority() {
        let admission = admit_build_agent_request(&request(false));
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"build_agent_request_dispatch_not_authorized".to_string())
        );
    }

    #[test]
    fn shell_or_arbitrary_build_actions_are_not_admitted() {
        let mut request = request(true);
        request.action = "bash -c make check".to_string();
        assert!(
            admit_build_agent_request(&request)
                .blockers
                .contains(&"build_agent_request_action_not_allowed".to_string())
        );
    }

    #[test]
    fn canonical_artifact_build_is_admissible() {
        let mut request = request(true);
        request.action = "make:build".to_string();
        assert_eq!(admit_build_agent_request(&request).status, "admitted");
    }

    #[test]
    fn canonical_release_build_is_admissible() {
        let mut request = request(true);
        request.action = "make:build-release".to_string();
        assert_eq!(admit_build_agent_request(&request).status, "admitted");
    }
}
