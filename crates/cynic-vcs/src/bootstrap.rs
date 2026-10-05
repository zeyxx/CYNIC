//! Source repository bootstrap reconciliation — pure decision layer.

use crate::types::*;

pub fn reconcile_source_repository_bootstrap(
    request: &SourceRepositoryBootstrapRequest,
    observation: &SourceRepositoryBootstrapObservation,
) -> SourceRepositoryBootstrapDecision {
    let mut blockers = Vec::new();
    if request.schema != SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA {
        blockers.push("source_bootstrap_request_schema_mismatch".to_string());
    }
    if observation.schema != SOURCE_REPOSITORY_BOOTSTRAP_OBSERVATION_SCHEMA {
        blockers.push("source_bootstrap_observation_schema_mismatch".to_string());
    }
    if request.node_id != observation.node_id || request.repository_id != observation.repository_id
    {
        blockers.push("source_bootstrap_identity_mismatch".to_string());
    }
    if request.source.trim().is_empty() || request.revision.trim().is_empty() {
        blockers.push("source_bootstrap_desired_state_incomplete".to_string());
    }
    if source_contains_inline_secret(&request.source) {
        blockers.push("source_bootstrap_source_contains_credentials".to_string());
    }
    if !observation.state_root_exists {
        blockers.push("source_bootstrap_state_root_missing".to_string());
    } else if !observation.state_root_is_directory {
        blockers.push("source_bootstrap_state_root_rejected".to_string());
    } else if !observation.state_root_owned_by_principal {
        blockers.push("source_bootstrap_state_root_ownership_mismatch".to_string());
    }
    if !observation.running_as_required_principal {
        blockers.push("source_bootstrap_principal_mismatch".to_string());
    }
    if !blockers.is_empty() {
        return SourceRepositoryBootstrapDecision::Blocked { blockers };
    }
    if !observation.unknowns.is_empty() {
        return SourceRepositoryBootstrapDecision::Unknown {
            unknowns: observation.unknowns.clone(),
        };
    }
    if !observation.repository_exists {
        return SourceRepositoryBootstrapDecision::BootstrapRequired;
    }
    if !observation.repository_is_git {
        return SourceRepositoryBootstrapDecision::Blocked {
            blockers: vec!["source_bootstrap_repository_not_git".to_string()],
        };
    }
    if observation.observed_source.as_deref() != Some(request.source.as_str()) {
        return SourceRepositoryBootstrapDecision::Blocked {
            blockers: vec!["source_bootstrap_source_drift".to_string()],
        };
    }
    if observation.observed_revision.as_deref() != Some(request.revision.as_str()) {
        return SourceRepositoryBootstrapDecision::SyncRequired;
    }
    SourceRepositoryBootstrapDecision::Converged
}

fn source_contains_inline_secret(source: &str) -> bool {
    let Some((scheme, remainder)) = source.split_once("://") else {
        return true;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    let Some((userinfo, _)) = authority.rsplit_once('@') else {
        return false;
    };
    scheme != "ssh" || userinfo.is_empty() || userinfo.contains(':')
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    fn request() -> SourceRepositoryBootstrapRequest {
        SourceRepositoryBootstrapRequest {
            schema: SOURCE_REPOSITORY_BOOTSTRAP_REQUEST_SCHEMA.into(),
            node_id: "builder".into(),
            repository_id: "cynic".into(),
            source: "ssh://forgejo@forgejo.synapse.cynic:22/owner/CYNIC.git".into(),
            revision: "a".repeat(40),
        }
    }

    fn observation() -> SourceRepositoryBootstrapObservation {
        SourceRepositoryBootstrapObservation {
            schema: SOURCE_REPOSITORY_BOOTSTRAP_OBSERVATION_SCHEMA.into(),
            node_id: "builder".into(),
            repository_id: "cynic".into(),
            repository_path: "/state/source/repositories/CYNIC".into(),
            required_principal: "cynic".into(),
            state_root_exists: true,
            state_root_is_directory: true,
            state_root_owned_by_principal: true,
            running_as_required_principal: true,
            repository_exists: true,
            repository_is_git: true,
            observed_revision: Some("b".repeat(40)),
            observed_source: Some(request().source),
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn revision_drift_is_a_reconcilable_sync() {
        assert_eq!(
            reconcile_source_repository_bootstrap(&request(), &observation()),
            SourceRepositoryBootstrapDecision::SyncRequired
        );
    }

    #[test]
    fn source_identity_drift_stays_blocked() {
        let mut observed = observation();
        observed.observed_source = Some("https://example.invalid/other.git".into());
        assert!(matches!(
            reconcile_source_repository_bootstrap(&request(), &observed),
            SourceRepositoryBootstrapDecision::Blocked { .. }
        ));
    }
}
