use crate::{
    SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA, SourceWorktreeInventoryEntry,
    SourceWorktreeInventoryObservation, SourceWorktreeInventoryUnknown,
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
            relative_path: Some("source/workspaces/cynic/ws-a".to_string()),
            head: Some("a".repeat(40)),
            observed_principal: Some("cynic".to_string()),
            path_is_symlink: Some(false),
            git_registered: Some(true),
            git_prunable: Some(false),
            detached_head: Some(false),
            clean: Some(true),
            unknowns: Vec::new(),
        }],
        evidence_refs: vec![DIGEST.to_string()],
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
fn prunable_fact_preserves_independent_cleanliness() {
    let mut observed = observation();
    observed.entries[0].git_prunable = Some(true);

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
