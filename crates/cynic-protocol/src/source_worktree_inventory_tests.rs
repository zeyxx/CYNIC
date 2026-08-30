use crate::{
    SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES, SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA,
    SourceWorktreeInventoryEntry, SourceWorktreeInventoryEntryState,
    SourceWorktreeInventoryObservation, SourceWorktreeInventoryObservationUnknown,
    SourceWorktreeInventoryPathBasis, SourceWorktreeInventoryUnknown,
    validate_source_worktree_inventory_observation,
};

const DIGEST: &str = "sha3-512:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn observation() -> SourceWorktreeInventoryObservation {
    SourceWorktreeInventoryObservation {
        schema: SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
        repository_id: "cynic".to_string(),
        node_id: "builder-a".to_string(),
        observed_at_ms: 1_000,
        entries: vec![SourceWorktreeInventoryEntry {
            candidate_id: DIGEST.to_string(),
            evidence_digest: DIGEST.to_string(),
            state: SourceWorktreeInventoryEntryState::Observed,
            unknown_attribute_count: 0,
            path_basis: SourceWorktreeInventoryPathBasis::WorkspaceLayout,
            relative_path: Some("source/workspaces/cynic/ws-a".to_string()),
            head: Some("a".repeat(40)),
            branch: Some("refs/heads/agent/ws-a".to_string()),
            observed_principal: Some("cynic".to_string()),
            path_is_symlink: Some(false),
            git_registered: Some(true),
            git_prunable: Some(false),
            locked: Some(false),
            bare: Some(false),
            lock_reason: None,
            prunable_reason: None,
            detached_head: Some(false),
            clean: Some(true),
            unknowns: Vec::new(),
        }],
        evidence_refs: vec![DIGEST.to_string()],
        observation_unknowns: Vec::new(),
    }
}

#[test]
fn valid_observation_preserves_git_facts_without_classifying_them() {
    let observed = observation();

    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
    assert_eq!(observed.entries[0].git_prunable, Some(false));
    assert_eq!(observed.entries[0].clean, Some(true));
}

#[test]
fn unknown_git_fact_is_preserved_as_a_typed_value() {
    let mut observed = observation();
    observed.entries[0].clean = None;
    observed.entries[0]
        .unknowns
        .push(SourceWorktreeInventoryUnknown::Cleanliness);

    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
    assert_eq!(observed.entries[0].clean, None);
    assert_eq!(
        observed.entries[0].unknowns,
        vec![SourceWorktreeInventoryUnknown::Cleanliness]
    );
}

#[test]
fn known_fact_cannot_also_be_marked_unknown() {
    let mut observed = observation();
    observed.entries[0]
        .unknowns
        .push(SourceWorktreeInventoryUnknown::Cleanliness);

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_unknown_contradictory".to_string())
    );
}

#[test]
fn unknown_git_attributes_are_explicitly_annotated() {
    let mut observed = observation();
    observed.entries[0].unknown_attribute_count = 1;
    observed.entries[0]
        .unknowns
        .push(SourceWorktreeInventoryUnknown::GitAttribute);

    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
}

#[test]
fn prunable_fact_preserves_independent_cleanliness() {
    let mut observed = observation();
    observed.entries[0].git_prunable = Some(true);

    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
}

#[test]
fn main_locked_and_bare_facts_are_representable() {
    let mut observed = observation();
    observed.entries[0].path_basis = SourceWorktreeInventoryPathBasis::MainRepository;
    observed.entries[0].relative_path = None;
    observed.entries[0].locked = Some(true);
    observed.entries[0].lock_reason = Some("recovery hold".to_string());
    observed.entries[0].bare = Some(true);

    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
}

#[test]
fn entry_evidence_must_be_linked_to_observation_evidence() {
    let mut observed = observation();
    observed.entries[0].evidence_digest =
        "sha3-512:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string();

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_entry_evidence_unlinked".to_string())
    );
}

#[test]
fn path_is_relative_and_path_free() {
    let mut observed = observation();
    observed.entries[0].relative_path = Some("/home/user/worktree".to_string());

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_absolute_path_forbidden".to_string())
    );
}

#[test]
fn traversal_path_is_rejected() {
    let mut observed = observation();
    observed.entries[0].relative_path = Some("source/../../operator".to_string());

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_path_traversal_forbidden".to_string())
    );
}

#[test]
fn drive_qualified_path_is_rejected_on_all_consumers() {
    let mut observed = observation();
    observed.entries[0].relative_path = Some("C:\\worktree".to_string());

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_absolute_path_forbidden".to_string())
    );
}

#[test]
fn empty_dot_and_control_paths_are_rejected() {
    for path in ["", ".", "source/\u{7f}worktree"] {
        let mut observed = observation();
        observed.entries[0].relative_path = Some(path.to_string());

        assert!(
            validate_source_worktree_inventory_observation(&observed)
                .contains(&"worktree_inventory_relative_path_invalid".to_string())
        );
    }
}

#[test]
fn windows_invalid_components_are_rejected() {
    for path in ["source/CON", "source/worktree:", "source/worktree."] {
        let mut observed = observation();
        observed.entries[0].relative_path = Some(path.to_string());

        assert!(
            validate_source_worktree_inventory_observation(&observed)
                .contains(&"worktree_inventory_relative_path_invalid".to_string())
        );
    }
}

#[test]
fn noncanonical_relative_path_aliases_are_rejected() {
    for path in ["source//ws", "source/./ws", "source/ws/"] {
        let mut observed = observation();
        observed.entries[0].relative_path = Some(path.to_string());

        assert!(
            validate_source_worktree_inventory_observation(&observed)
                .contains(&"worktree_inventory_relative_path_invalid".to_string())
        );
    }
}

#[test]
fn duplicate_candidate_ids_are_rejected() {
    let mut observed = observation();
    observed.entries.push(observed.entries[0].clone());

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_candidate_id_duplicate".to_string())
    );
}

#[test]
fn malformed_identity_and_evidence_are_rejected() {
    let mut observed = observation();
    observed.repository_id = "../other".to_string();
    observed.node_id = "/foreign".to_string();
    observed.entries[0].candidate_id = "not-a-digest".to_string();

    let violations = validate_source_worktree_inventory_observation(&observed);
    assert!(violations.contains(&"worktree_inventory_repository_id_invalid".to_string()));
    assert!(violations.contains(&"worktree_inventory_node_id_invalid".to_string()));
    assert!(violations.contains(&"worktree_inventory_entry_evidence_invalid".to_string()));
}

#[test]
fn dot_control_and_uppercase_digest_values_are_rejected() {
    let mut observed = observation();
    observed.repository_id = ".".to_string();
    observed.entries[0].relative_path = Some("source\u{1f}worktree".to_string());
    observed.entries[0].evidence_digest = DIGEST.to_ascii_uppercase();

    let violations = validate_source_worktree_inventory_observation(&observed);
    assert!(violations.contains(&"worktree_inventory_repository_id_invalid".to_string()));
    assert!(violations.contains(&"worktree_inventory_relative_path_invalid".to_string()));
    assert!(violations.contains(&"worktree_inventory_entry_evidence_invalid".to_string()));
}

#[test]
fn serialized_observation_contains_no_machine_absolute_path_by_contract() {
    let encoded = serde_json::to_string(&observation()).expect("observation serializes");
    assert!(!encoded.contains("/home/"));
    assert!(!encoded.contains("/state/"));
}

#[test]
fn empty_observation_requires_an_explicit_coverage_unknown() {
    let mut observed = observation();
    observed.entries.clear();
    observed.evidence_refs.clear();

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_coverage_unknown_unannotated".to_string())
    );

    observed
        .observation_unknowns
        .push(SourceWorktreeInventoryObservationUnknown::GitCommandFailed);
    assert!(validate_source_worktree_inventory_observation(&observed).is_empty());
}

#[test]
fn validator_does_not_inspect_entries_beyond_the_declared_limit() {
    let mut observed = observation();
    let template = observed.entries.remove(0);
    observed.evidence_refs.clear();

    for index in 0..=SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES {
        let digest = format!("sha3-512:{index:0128x}");
        let mut entry = template.clone();
        entry.candidate_id = digest.clone();
        entry.evidence_digest = digest.clone();
        observed.entries.push(entry);
        observed.evidence_refs.push(digest);
    }
    observed.entries[SOURCE_WORKTREE_INVENTORY_MAX_ENTRIES].relative_path =
        Some("/outside/declared/limit".to_string());

    let violations = validate_source_worktree_inventory_observation(&observed);
    assert!(violations.contains(&"worktree_inventory_entries_limit_exceeded".to_string()));
    assert!(!violations.contains(&"worktree_inventory_absolute_path_forbidden".to_string()));
}

#[test]
fn validator_does_not_inspect_evidence_beyond_the_declared_limit() {
    let mut observed = observation();
    observed.evidence_refs.clear();
    observed.evidence_refs.push(DIGEST.to_string());
    for index in 1..8192 {
        observed
            .evidence_refs
            .push(format!("sha3-512:{index:0128x}"));
    }
    observed.evidence_refs.push("not-a-digest".to_string());

    let violations = validate_source_worktree_inventory_observation(&observed);
    assert!(violations.contains(&"worktree_inventory_evidence_limit_exceeded".to_string()));
    assert!(!violations.contains(&"worktree_inventory_evidence_invalid".to_string()));
}

#[test]
fn entry_unknowns_are_rejected_beyond_the_typed_variant_limit() {
    let mut observed = observation();
    observed.entries[0].clean = None;
    observed.entries[0].unknowns = vec![SourceWorktreeInventoryUnknown::Cleanliness; 15];

    assert!(
        validate_source_worktree_inventory_observation(&observed)
            .contains(&"worktree_inventory_unknown_limit_exceeded".to_string())
    );
}
