use cynic_protocol::{
    DevelopmentGenesisBuildAdmission, DevelopmentGenesisBuildRequest,
    SourceMaterializationObservation, SourceMaterializationPlan, SourceMaterializationRequest,
    admit_development_genesis_build, development_genesis_canonical_argv,
    plan_source_materialization,
};

pub const DEVELOPMENT_GENESIS_BUILD_INVOCATION_SCHEMA: &str =
    "cynic.host.development_genesis.build_invocation.v1";
pub const DEVELOPMENT_GENESIS_BUILD_RECEIPT_SCHEMA: &str =
    "cynic.host.development_genesis.build_receipt.v1";

/// The canonical, adapter-facing invocation for an admitted DevelopmentGenesis
/// request. The caller supplies no argv: it is derived exclusively from the
/// protocol action allowlist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentGenesisBuildInvocation {
    pub schema: String,
    pub request_id: String,
    pub source_commit: String,
    pub source_tree: String,
    pub source_authority_ref: String,
    pub builder_image: String,
    pub argv: Vec<Vec<String>>,
}

/// Adapter read-back after a bounded builder invocation. An adapter must echo
/// the argv it executed; mismatches are not promotable to a receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentGenesisBuilderOutput {
    pub executed_argv: Vec<Vec<String>>,
    pub artifact_digests: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub trait DevelopmentGenesisBuildPort {
    fn run(
        &self,
        invocation: &DevelopmentGenesisBuildInvocation,
    ) -> Result<DevelopmentGenesisBuilderOutput, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentGenesisBuildReceipt {
    pub schema: String,
    pub request_id: String,
    pub source_commit: String,
    pub source_tree: String,
    pub source_authority_ref: String,
    pub builder_image: String,
    pub executed_argv: Vec<Vec<String>>,
    pub artifact_digests: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevelopmentGenesisBuildOutcome {
    Blocked {
        admission: DevelopmentGenesisBuildAdmission,
        source_materialization: Option<SourceMaterializationPlan>,
        blockers: Vec<String>,
    },
    Failed {
        request_id: String,
        reason: String,
    },
    Executed(DevelopmentGenesisBuildReceipt),
}

impl DevelopmentGenesisBuildOutcome {
    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked { .. })
    }

    #[cfg(test)]
    fn expect_receipt(self) -> DevelopmentGenesisBuildReceipt {
        match self {
            Self::Executed(receipt) => receipt,
            Self::Blocked { blockers, .. } => panic!("request blocked: {blockers:?}"),
            Self::Failed { reason, .. } => panic!("builder failed: {reason}"),
        }
    }
}

pub fn execute_development_genesis_build(
    request: &DevelopmentGenesisBuildRequest,
    source_request: &SourceMaterializationRequest,
    source_observation: &SourceMaterializationObservation,
    now_unix_s: u64,
    port: &dyn DevelopmentGenesisBuildPort,
) -> DevelopmentGenesisBuildOutcome {
    let admission = admit_development_genesis_build(request, now_unix_s);
    if admission.status != "admitted" {
        return DevelopmentGenesisBuildOutcome::Blocked {
            blockers: admission.blockers.clone(),
            admission,
            source_materialization: None,
        };
    }
    let source_plan = plan_source_materialization(
        source_request,
        source_observation,
        now_unix_s.saturating_mul(1_000),
    );
    let mut source_blockers = source_plan.blockers.clone();
    if source_request.source_commit != request.source_commit {
        source_blockers.push("development_genesis_source_commit_mismatch".to_string());
    }
    if source_request.source_tree.as_deref() != Some(request.source_tree.as_str()) {
        source_blockers.push("development_genesis_source_tree_mismatch".to_string());
    }
    if !request
        .evidence_refs
        .iter()
        .any(|reference| reference == &format!("source-capsule:{}", source_request.snapshot_digest))
    {
        source_blockers.push("development_genesis_source_capsule_mismatch".to_string());
    }
    if source_plan.status != "converged" {
        source_blockers.push(format!(
            "development_genesis_source_materialization_not_converged:{}",
            source_plan.status
        ));
    }
    source_blockers.extend(source_plan.unknowns.iter().cloned());
    source_blockers.sort();
    source_blockers.dedup();
    if !source_blockers.is_empty() {
        return DevelopmentGenesisBuildOutcome::Blocked {
            admission,
            source_materialization: Some(source_plan),
            blockers: source_blockers,
        };
    }
    let Some(argv) = development_genesis_canonical_argv(&request.allowed_actions) else {
        return DevelopmentGenesisBuildOutcome::Failed {
            request_id: request.request_id.clone(),
            reason: "admitted_development_genesis_request_lost_canonical_argv".to_string(),
        };
    };
    let invocation = DevelopmentGenesisBuildInvocation {
        schema: DEVELOPMENT_GENESIS_BUILD_INVOCATION_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        source_commit: request.source_commit.clone(),
        source_tree: request.source_tree.clone(),
        source_authority_ref: request.source_authority_ref.clone(),
        builder_image: request.builder_image.clone(),
        argv,
    };
    let output = match port.run(&invocation) {
        Ok(output) => output,
        Err(reason) => {
            return DevelopmentGenesisBuildOutcome::Failed {
                request_id: request.request_id.clone(),
                reason,
            };
        }
    };
    if output.executed_argv != invocation.argv {
        return DevelopmentGenesisBuildOutcome::Failed {
            request_id: request.request_id.clone(),
            reason: "development_genesis_builder_read_back_argv_mismatch".to_string(),
        };
    }
    let mut evidence_refs = request.evidence_refs.clone();
    evidence_refs.extend(output.evidence_refs);
    DevelopmentGenesisBuildOutcome::Executed(DevelopmentGenesisBuildReceipt {
        schema: DEVELOPMENT_GENESIS_BUILD_RECEIPT_SCHEMA.to_string(),
        request_id: request.request_id.clone(),
        source_commit: request.source_commit.clone(),
        source_tree: request.source_tree.clone(),
        source_authority_ref: request.source_authority_ref.clone(),
        builder_image: request.builder_image.clone(),
        executed_argv: invocation.argv,
        artifact_digests: output.artifact_digests,
        evidence_refs,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use cynic_protocol::development_genesis::{
        DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA, DevelopmentGenesisAction,
        DevelopmentGenesisBuildRequest,
    };
    use cynic_protocol::{
        SOURCE_MATERIALIZATION_FORMAT, SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA,
        SOURCE_MATERIALIZATION_REQUEST_SCHEMA, SecurityEvidenceState,
        SourceMaterializationObservation, SourceMaterializationRequest,
    };

    #[derive(Default)]
    struct RecordingPort {
        calls: Cell<usize>,
    }

    impl DevelopmentGenesisBuildPort for RecordingPort {
        fn run(
            &self,
            invocation: &DevelopmentGenesisBuildInvocation,
        ) -> Result<DevelopmentGenesisBuilderOutput, String> {
            self.calls.set(self.calls.get() + 1);
            Ok(DevelopmentGenesisBuilderOutput {
                executed_argv: invocation.argv.clone(),
                artifact_digests: vec![format!("sha256:{}", "e".repeat(64))],
                evidence_refs: vec!["builder-proof:sha256:ok".to_string()],
            })
        }
    }

    fn request() -> DevelopmentGenesisBuildRequest {
        DevelopmentGenesisBuildRequest {
            schema: DEVELOPMENT_GENESIS_BUILD_REQUEST_SCHEMA.to_string(),
            request_id: "development-genesis-builder-1".to_string(),
            source_commit: "a".repeat(40),
            source_tree: "b".repeat(40),
            source_authority_ref: "forgejo:cynic@aaaaaaaa".to_string(),
            builder_image: format!("docker.io/library/rust@sha256:{}", "c".repeat(64)),
            expires_at_unix_s: 200,
            allowed_actions: vec![
                DevelopmentGenesisAction::MakeCheck,
                DevelopmentGenesisAction::BuildCli,
                DevelopmentGenesisAction::BuildMcp,
            ],
            requested_capabilities: vec!["build_artifact".to_string()],
            evidence_refs: vec![format!("source-capsule:sha256:{}", "d".repeat(64))],
        }
    }

    fn source_request() -> SourceMaterializationRequest {
        SourceMaterializationRequest {
            schema: SOURCE_MATERIALIZATION_REQUEST_SCHEMA.to_string(),
            release_id: "development-genesis-builder-1".to_string(),
            repository_id: "cynic".to_string(),
            source_commit: "a".repeat(40),
            source_tree: Some("b".repeat(40)),
            snapshot_digest: format!("sha256:{}", "d".repeat(64)),
            snapshot_format: SOURCE_MATERIALIZATION_FORMAT.to_string(),
            target_node_id: "cynic-builder".to_string(),
            target_relative_path: "source/development-genesis-builder-1".to_string(),
            required_principal: "cynic".to_string(),
            retention_until_unix_ms: 200_000,
        }
    }

    fn source_observation(target_exists: bool) -> SourceMaterializationObservation {
        SourceMaterializationObservation {
            schema: SOURCE_MATERIALIZATION_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_ms: 100_000,
            target_node_id: "cynic-builder".to_string(),
            target_role: Some("build_runner".to_string()),
            node_identity: SecurityEvidenceState::Proven,
            running_as_required_principal: SecurityEvidenceState::Proven,
            snapshot_present: true,
            snapshot_digest_read_back: Some(format!("sha256:{}", "d".repeat(64))),
            snapshot_format_read_back: Some(SOURCE_MATERIALIZATION_FORMAT.to_string()),
            target_exists,
            target_is_isolated: SecurityEvidenceState::Proven,
            observed_commit: target_exists.then(|| "a".repeat(40)),
            observed_tree: target_exists.then(|| "b".repeat(40)),
            observed_snapshot_digest: target_exists.then(|| format!("sha256:{}", "d".repeat(64))),
            evidence_refs: vec!["source-materialization:read-back".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn admitted_request_runs_only_canonical_argv_and_retains_lineage() {
        let port = RecordingPort::default();
        let outcome = execute_development_genesis_build(
            &request(),
            &source_request(),
            &source_observation(true),
            100,
            &port,
        );

        let receipt = outcome.expect_receipt();
        assert_eq!(port.calls.get(), 1);
        assert_eq!(
            receipt.executed_argv,
            vec![
                vec!["make", "check"],
                vec!["cargo", "build", "--release", "-p", "cynic-cli"],
                vec!["cargo", "build", "--release", "-p", "cynic-mcp"],
            ]
        );
        assert_eq!(receipt.source_commit, "a".repeat(40));
        assert_eq!(receipt.source_tree, "b".repeat(40));
        assert!(receipt.builder_image.contains("@sha256:"));
        assert!(
            receipt
                .evidence_refs
                .iter()
                .any(|reference| reference.starts_with("source-capsule:sha256:"))
        );
    }

    #[test]
    fn blocked_request_never_reaches_builder_port() {
        let port = RecordingPort::default();
        let mut request = request();
        request.expires_at_unix_s = 100;

        let outcome = execute_development_genesis_build(
            &request,
            &source_request(),
            &source_observation(true),
            100,
            &port,
        );
        assert!(outcome.is_blocked());
        assert_eq!(port.calls.get(), 0);
    }

    #[test]
    fn planned_source_materialization_never_reaches_builder_port() {
        let port = RecordingPort::default();
        let outcome = execute_development_genesis_build(
            &request(),
            &source_request(),
            &source_observation(false),
            100,
            &port,
        );

        assert!(outcome.is_blocked());
        assert_eq!(port.calls.get(), 0);
    }
}
