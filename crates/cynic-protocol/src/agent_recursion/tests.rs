use super::*;

fn evidence(subject: &str) -> AgentEvidenceRef {
    AgentEvidenceRef {
        kind: "runtime_proof".to_string(),
        subject: subject.to_string(),
        digest: format!("sha256:{}", "a".repeat(64)),
        lineage: None,
    }
}

fn parent() -> AgentDelegationEnvelope {
    AgentDelegationEnvelope {
        schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "root-task".to_string(),
        node_id: "node-a".to_string(),
        agent_id: "agent-root".to_string(),
        parent_agent_id: None,
        depth: 0,
        objective: "reconcile local node".to_string(),
        authority_snapshot: evidence("foundation"),
        context_refs: vec![evidence("node-observation")],
        capabilities: vec![AgentCapabilityGrant {
            id: "runtime.observe".to_string(),
            mutates: false,
        }],
        budget: AgentBudget {
            context_tokens: 100_000,
            actions: 10,
            children: 4,
            wall_time_ms: 60_000,
            cpu_ms: None,
        },
        expected_proofs: vec!["capability_observation".to_string()],
    }
}

fn child() -> AgentDelegationEnvelope {
    AgentDelegationEnvelope {
        schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: "child-task".to_string(),
        node_id: "node-a".to_string(),
        agent_id: "agent-child-1".to_string(),
        parent_agent_id: Some("agent-root".to_string()),
        depth: 1,
        objective: "reconcile delegated subtree".to_string(),
        authority_snapshot: evidence("foundation"),
        context_refs: vec![evidence("parent-observation")],
        capabilities: vec![AgentCapabilityGrant {
            id: "runtime.observe".to_string(),
            mutates: false,
        }],
        budget: AgentBudget {
            context_tokens: 50_000,
            actions: 5,
            children: 2,
            wall_time_ms: 30_000,
            cpu_ms: None,
        },
        expected_proofs: vec!["capability_observation".to_string()],
    }
}

#[test]
fn admits_bounded_same_node_child() {
    assert!(admit_child_delegation(&parent(), &child()).admitted());
}

#[test]
fn denies_cross_node_or_amplified_child() {
    let parent = parent();
    let mut child = child();
    child.node_id = "node-b".to_string();
    child.budget.actions = 11;
    child.capabilities[0].mutates = true;

    let report = admit_child_delegation(&parent, &child);
    let AgentRecursionDecision::Deny { violations } = report.decision else {
        panic!("amplified child must be denied");
    };
    let codes = violations
        .iter()
        .map(|violation| violation.code.as_str())
        .collect::<BTreeSet<_>>();
    assert!(codes.contains("node_boundary_crossed"));
    assert!(codes.contains("budget_amplification"));
    assert!(codes.contains("mutation_authority_amplification"));
}

#[test]
fn changed_authority_requires_subset_proof() {
    let parent = parent();
    let mut child = child();
    child.authority_snapshot.digest = format!("sha256:{}", "b".repeat(64));
    assert!(matches!(
        admit_child_delegation(&parent, &child).decision,
        AgentRecursionDecision::Unknown { missing_evidence }
            if missing_evidence == vec!["authority_subset_proof"]
    ));
}

#[test]
fn completion_returns_proof_upward_within_budget() {
    let child = child();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: child.delegation_id.clone(),
        node_id: child.node_id.clone(),
        agent_id: child.agent_id.clone(),
        parent_agent_id: child.parent_agent_id.clone(),
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![evidence("capability-observation")],
        unknowns: Vec::new(),
        consumed_budget: AgentBudget {
            context_tokens: 12_000,
            actions: 1,
            children: 0,
            wall_time_ms: 4_000,
            cpu_ms: None,
        },
    };
    assert!(admit_completion(&child, &completion).admitted());
}

#[test]
fn completed_without_proof_is_denied() {
    let child = child();
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: child.delegation_id.clone(),
        node_id: child.node_id.clone(),
        agent_id: child.agent_id.clone(),
        parent_agent_id: child.parent_agent_id.clone(),
        status: AgentCompletionStatus::Completed,
        evidence_refs: Vec::new(),
        unknowns: Vec::new(),
        consumed_budget: child.budget.clone(),
    };
    assert!(!admit_completion(&child, &completion).admitted());
}

#[test]
fn malformed_declared_lineage_is_denied_at_admission() {
    let child = child();
    let mut invalid = evidence("invalid-lineage");
    invalid.lineage = Some(AgentEvidenceLineage {
        producer_id: "producer-a".to_string(),
        producer_attestation_digest: format!("sha256:{}", "a".repeat(64)),
        origin_digests: Vec::new(),
        derivation_digests: Vec::new(),
        influence_digests: Vec::new(),
    });
    let completion = AgentCompletionEnvelope {
        schema: AGENT_COMPLETION_ENVELOPE_SCHEMA.to_string(),
        delegation_id: child.delegation_id.clone(),
        node_id: child.node_id.clone(),
        agent_id: child.agent_id.clone(),
        parent_agent_id: child.parent_agent_id.clone(),
        status: AgentCompletionStatus::Completed,
        evidence_refs: vec![invalid],
        unknowns: Vec::new(),
        consumed_budget: child.budget.clone(),
    };
    assert!(matches!(
        admit_completion(&child, &completion).decision,
        AgentRecursionDecision::Deny { violations }
            if violations.iter().any(|violation| violation.code == "invalid_evidence_lineage")
    ));
}

#[test]
fn validate_evidence_refs_reports_invalid_entries() {
    let mut bad = evidence("bad");
    bad.digest = "not-a-digest".into();
    let violations = validate_evidence_refs(std::slice::from_ref(&bad));
    assert!(
        violations
            .iter()
            .any(|violation| violation.code == "invalid_evidence_ref")
    );
}
