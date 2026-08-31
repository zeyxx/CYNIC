mod w1_b_tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};
    use cynic_protocol::validate_source_worktree_inventory_observation;

    fn sha3(digest: &str) -> String {
        format!("sha3-512:{digest}")
    }

    fn porcelain_output() -> String {
        let main = sha3(&"a".repeat(128));
        let agent = sha3(&"b".repeat(128));
        format!(
            "worktree /var/lib/cynic/source/repositories/CYNIC\0HEAD {main}\0branch refs/heads/main\0\0worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {agent}\0branch refs/heads/agent/ws-a\0locked agent running\0\0"
        )
    }

    #[test]
    fn adapter_produces_observation_from_porcelain_output() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.repository_id, "cynic");
        assert_eq!(observation.node_id, "builder-a");
        assert_eq!(observation.entries.len(), 2);
        assert!(observation.evidence_refs.len() >= 2);
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_marks_main_and_workspace_path_basis() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(
            observation.entries[0].path_basis,
            SourceWorktreeInventoryPathBasis::MainRepository
        );
        assert_eq!(
            observation.entries[1].path_basis,
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        );
    }

    #[test]
    fn parser_uses_injected_layout_and_marks_outside_paths() {
        let root = tempfile::tempdir().unwrap();
        let repository = root.path().join("repositories/CYNIC");
        let workspace_store = root.path().join("workspaces/cynic");
        let workspace = workspace_store.join("ws-a");
        let outside = root.path().join("foreign/ws-b");
        std::fs::create_dir_all(&repository).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let head = sha3(&"a".repeat(128));
        let stdout = format!(
            "worktree {}\0HEAD {head}\0branch refs/heads/main\0\0worktree {}\0HEAD {head}\0branch refs/heads/agent/ws-a\0\0worktree {}\0HEAD {head}\0branch refs/heads/foreign\0\0",
            repository.display(),
            workspace.display(),
            outside.display(),
        );

        let (entries, entry_limit_exceeded, _malformed_record_evidence) =
            parse_porcelain(&stdout, &repository, &workspace_store);

        assert!(!entry_limit_exceeded);
        assert_eq!(
            entries[0].path_basis,
            SourceWorktreeInventoryPathBasis::MainRepository
        );
        assert_eq!(
            entries[0].state,
            SourceWorktreeInventoryEntryState::Observed
        );
        assert_eq!(
            entries[1].path_basis,
            SourceWorktreeInventoryPathBasis::WorkspaceLayout
        );
        assert_eq!(entries[1].relative_path.as_deref(), Some("ws-a"));
        assert_eq!(
            entries[1].state,
            SourceWorktreeInventoryEntryState::Observed
        );
        assert_eq!(
            entries[2].path_basis,
            SourceWorktreeInventoryPathBasis::Unknown
        );
        assert_eq!(entries[2].relative_path, None);
        assert_eq!(
            entries[2].state,
            SourceWorktreeInventoryEntryState::OutsideDeclaredLayout
        );
        assert!(
            entries[2]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::RelativePath)
        );
    }

    #[test]
    fn adapter_treats_absent_porcelain_flags_as_false() {
        let (entries, entry_limit_exceeded, _malformed_record_evidence) = parse_porcelain(
            &porcelain_output(),
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );

        assert!(!entry_limit_exceeded);
        assert_eq!(entries[0].locked, Some(false));
        assert_eq!(entries[0].bare, Some(false));
        assert_eq!(entries[0].detached_head, Some(false));
        assert_eq!(entries[0].git_prunable, Some(false));
        assert_eq!(entries[1].locked, Some(true));
        assert_eq!(entries[1].bare, Some(false));
        assert_eq!(entries[1].detached_head, Some(false));
        assert_eq!(entries[1].git_prunable, Some(false));
    }

    #[test]
    fn detached_head_makes_branch_absence_known() {
        let head = sha3(&"a".repeat(128));
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: format!(
                        "worktree /var/lib/cynic/source/workspaces/cynic/ws-detached\0HEAD {head}\0detached\0\0"
                    ),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries[0].branch, None);
        assert_eq!(observation.entries[0].detached_head, Some(true));
        assert!(
            !observation.entries[0]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::Branch)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_records_locked_fact() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: porcelain_output(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries[1].locked, Some(true));
        assert_eq!(
            observation.entries[1].lock_reason,
            Some("agent running".to_string())
        );
    }

    #[test]
    fn adapter_preserves_nul_delimited_porcelain_values() {
        let head = sha3(&"a".repeat(128));
        let stdout = format!(
            "worktree /var/lib/cynic/source/repositories/CYNIC\0HEAD {head}\0branch refs/heads/main\0locked first line\nsecond line\0\0"
        );
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout,
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries.len(), 1);
        assert_eq!(
            observation.entries[0].lock_reason,
            Some("first line\nsecond line".to_string())
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn non_nul_output_is_malformed_record_evidence() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: "worktree /tmp/ws\nHEAD abc\n".to_string(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert!(observation.entries.is_empty());
        assert_eq!(observation.evidence_refs.len(), 1);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::MalformedRecord]
        );
    }

    #[test]
    fn only_complete_stdout_records_are_observed() {
        let warning_runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: "warning without porcelain".to_string(),
                    spawn_error: None,
                },
            )],
        };
        let warning_observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &warning_runner,
        );
        assert_eq!(
            warning_observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::EmptyOutput]
        );

        let head = sha3(&"a".repeat(128));
        let truncated =
            format!("worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0");
        let (entries, _, malformed_evidence) = parse_porcelain(
            &truncated,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );
        assert!(entries.is_empty());
        assert_eq!(malformed_evidence.len(), 1);
    }

    #[test]
    fn non_utf8_stdout_is_bound_as_malformed_raw_evidence() {
        struct InvalidUtf8Runner;
        impl SourceWorktreeInventoryRunner for InvalidUtf8Runner {
            fn run(&self, _argv: &[&str]) -> SourceWorktreeInventoryCommandOutput {
                SourceWorktreeInventoryCommandOutput {
                    exit_code: Some(0),
                    stdout: b"worktree /tmp/"
                        .iter()
                        .copied()
                        .chain([0xff, 0, 0])
                        .collect(),
                    stderr: Vec::new(),
                    spawn_error: None,
                }
            }
        }

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &InvalidUtf8Runner,
        );

        assert!(observation.entries.is_empty());
        assert_eq!(observation.evidence_refs.len(), 1);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::MalformedRecord]
        );
    }

    #[test]
    fn adapter_preserves_unknown_porcelain_attributes() {
        let text = porcelain_output().replace(
            "locked agent running\0\0",
            "locked agent running\0future-attribute opaque-value\0\0",
        );
        let (entries, entry_limit_exceeded, _malformed_record_evidence) = parse_porcelain(
            &text,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );
        assert!(!entry_limit_exceeded);
        let evidence_refs = entries
            .iter()
            .map(|entry| entry.evidence_digest.clone())
            .collect();
        let observation = SourceWorktreeInventoryObservation {
            schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: "cynic".to_string(),
            node_id: "builder-a".to_string(),
            observed_at_ms: 1,
            entries,
            evidence_refs,
            observation_unknowns: Vec::new(),
        };

        assert_eq!(observation.entries[1].unknown_attribute_count, 1);
        assert!(
            observation.entries[1]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::GitAttribute)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn unknown_porcelain_content_changes_the_evidence_digest() {
        let head = sha3(&"a".repeat(128));
        let first = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0future-alpha opaque-a\0\0"
        );
        let second = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0future-beta opaque-b\0\0"
        );
        let repository = Path::new("/var/lib/cynic/source/repositories/CYNIC");
        let workspace_store = Path::new("/var/lib/cynic/source/workspaces/cynic");

        let (first_entries, first_limit, first_malformed_record_evidence) =
            parse_porcelain(&first, repository, workspace_store);
        let (second_entries, second_limit, second_malformed_record_evidence) =
            parse_porcelain(&second, repository, workspace_store);

        assert!(!first_limit);
        assert!(!second_limit);
        assert!(first_malformed_record_evidence.is_empty());
        assert!(second_malformed_record_evidence.is_empty());
        assert_eq!(first_entries[0].unknown_attribute_count, 1);
        assert_eq!(second_entries[0].unknown_attribute_count, 1);
        assert_ne!(
            first_entries[0].evidence_digest,
            second_entries[0].evidence_digest
        );
    }

    #[test]
    fn unknown_attribute_counter_saturation_marks_record_malformed() {
        let head = sha3(&"a".repeat(128));
        let mut text =
            format!("worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0");
        for _ in 0..=u16::MAX {
            text.push_str("x\0");
        }
        text.push('\0');

        let (entries, entry_limit_exceeded, malformed_record_evidence) = parse_porcelain(
            &text,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );

        assert!(!entry_limit_exceeded);
        assert!(malformed_record_evidence.is_empty());
        assert_eq!(entries[0].unknown_attribute_count, u16::MAX);
        assert_eq!(
            entries[0].state,
            SourceWorktreeInventoryEntryState::Malformed
        );
    }

    #[test]
    fn oversized_porcelain_fields_emit_valid_malformed_entries() {
        let head = sha3(&"a".repeat(128));
        let oversized = "x".repeat(4097);
        let records = [
            format!("worktree /var/lib/cynic/source/workspaces/cynic/{oversized}\0HEAD {head}\0\0"),
            format!("worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {oversized}\0\0"),
            format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0branch {oversized}\0\0"
            ),
            format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0locked {oversized}\0\0"
            ),
            format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0prunable {oversized}\0\0"
            ),
        ];

        for text in records {
            let (entries, entry_limit_exceeded, malformed_record_evidence) = parse_porcelain(
                &text,
                Path::new("/var/lib/cynic/source/repositories/CYNIC"),
                Path::new("/var/lib/cynic/source/workspaces/cynic"),
            );
            assert!(!entry_limit_exceeded);
            assert!(malformed_record_evidence.is_empty());
            assert_eq!(entries.len(), 1);
            assert_eq!(
                entries[0].state,
                SourceWorktreeInventoryEntryState::Malformed
            );

            let observation = SourceWorktreeInventoryObservation {
                schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
                repository_id: "cynic".to_string(),
                node_id: "builder-a".to_string(),
                observed_at_ms: 1,
                evidence_refs: vec![entries[0].evidence_digest.clone()],
                entries,
                observation_unknowns: Vec::new(),
            };
            assert!(
                validate_source_worktree_inventory_observation(&observation).is_empty(),
                "oversized record must degrade to a valid malformed entry"
            );
        }
    }

    #[test]
    fn duplicate_known_attribute_marks_the_record_malformed() {
        let first_head = sha3(&"a".repeat(128));
        let second_head = sha3(&"b".repeat(128));
        let text = format!(
            "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {first_head}\0HEAD {second_head}\0\0"
        );

        let (entries, entry_limit_exceeded, _malformed_record_evidence) = parse_porcelain(
            &text,
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
        );
        let evidence_refs = entries
            .iter()
            .map(|entry| entry.evidence_digest.clone())
            .collect();
        let observation = SourceWorktreeInventoryObservation {
            schema: cynic_protocol::SOURCE_WORKTREE_INVENTORY_OBSERVATION_SCHEMA.to_string(),
            repository_id: "cynic".to_string(),
            node_id: "builder-a".to_string(),
            observed_at_ms: 1,
            entries,
            evidence_refs,
            observation_unknowns: Vec::new(),
        };

        assert!(!entry_limit_exceeded);
        assert_eq!(
            observation.entries[0].state,
            SourceWorktreeInventoryEntryState::Malformed
        );
        assert_eq!(observation.entries[0].head, None);
        assert_eq!(observation.entries[0].unknown_attribute_count, 1);
        assert!(
            observation.entries[0]
                .unknowns
                .contains(&SourceWorktreeInventoryUnknown::GitHead)
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn every_singleton_porcelain_field_rejects_duplicates() {
        let head = sha3(&"a".repeat(128));
        for duplicate_fields in [
            "branch refs/heads/one\0branch refs/heads/two",
            "bare\0bare",
            "locked\0locked reason",
            "detached\0detached",
            "prunable\0prunable reason",
        ] {
            let text = format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0{duplicate_fields}\0\0"
            );
            let (entries, entry_limit_exceeded, _malformed_record_evidence) = parse_porcelain(
                &text,
                Path::new("/var/lib/cynic/source/repositories/CYNIC"),
                Path::new("/var/lib/cynic/source/workspaces/cynic"),
            );

            assert!(!entry_limit_exceeded);
            assert_eq!(
                entries[0].state,
                SourceWorktreeInventoryEntryState::Malformed,
                "duplicate fields: {duplicate_fields}"
            );
            assert_eq!(
                entries[0].unknown_attribute_count, 1,
                "duplicate fields: {duplicate_fields}"
            );
        }
    }

    #[test]
    fn attribute_outside_a_record_is_global_malformed_evidence() {
        let head = sha3(&"a".repeat(128));
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: format!(
                        "worktree /var/lib/cynic/source/workspaces/cynic/ws-a\0HEAD {head}\0\0HEAD orphaned\0"
                    ),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries.len(), 1);
        assert_eq!(observation.entries[0].unknown_attribute_count, 0);
        assert_eq!(observation.evidence_refs.len(), 2);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::MalformedRecord]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_returns_empty_when_git_fails() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(128),
                    stdout: String::new(),
                    stderr: "fatal: not a git repository".to_string(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
        assert!(observation.evidence_refs.is_empty());
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::GitCommandFailed]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_rejects_oversized_porcelain_before_parsing() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: "x".repeat(4 * 1024 * 1024 + 1),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert!(observation.entries.is_empty());
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::OutputLimitExceeded]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_stops_at_the_entry_limit() {
        let head = sha3(&"a".repeat(128));
        let mut stdout = String::new();
        for index in 0..=4096 {
            stdout.push_str(&format!(
                "worktree /var/lib/cynic/source/workspaces/cynic/ws-{index}\0HEAD {head}\0branch refs/heads/agent/ws-{index}\0\0"
            ));
        }
        assert!(stdout.len() < MAX_PORCELAIN_BYTES);
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout,
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };

        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );

        assert_eq!(observation.entries.len(), 4096);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::EntryLimitExceeded]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }

    #[test]
    fn adapter_handles_empty_repository_with_no_worktrees() {
        let runner = MapArgvRunner {
            responses: vec![(
                vec![
                    "git".to_string(),
                    "-C".to_string(),
                    "/var/lib/cynic/source/repositories/CYNIC".to_string(),
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                    "-z".to_string(),
                ],
                ArgvOutput {
                    exit_code: Some(0),
                    stdout: String::new(),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        };
        let observation = observe_source_worktree_inventory(
            Path::new("/var/lib/cynic/source/repositories/CYNIC"),
            Path::new("/var/lib/cynic/source/workspaces/cynic"),
            "cynic",
            "builder-a",
            &runner,
        );
        assert_eq!(observation.entries.len(), 0);
        assert_eq!(
            observation.observation_unknowns,
            vec![SourceWorktreeInventoryObservationUnknown::EmptyOutput]
        );
        assert!(validate_source_worktree_inventory_observation(&observation).is_empty());
    }
}
