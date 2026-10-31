//! Node-owned translation boundary for typed Builder slice actions.
//!
//! Re-exports canonical command translation from `cynic_build::build_slice`.

pub use cynic_build::build_slice::{build_slice_cargo_argv, BuildSliceCommandError};

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_build::build_slice::{
        BuildSliceAction, BuildSliceNetworkMode, BuildSlicePhase, BuildSliceScope,
        BuildSliceTarget, BUILD_SLICE_ACTION_SCHEMA,
    };

    fn action(phase: BuildSlicePhase) -> BuildSliceAction {
        BuildSliceAction {
            schema: BUILD_SLICE_ACTION_SCHEMA.to_string(),
            source_commit: "a".repeat(40),
            phase,
            scope: if matches!(phase, BuildSlicePhase::Test) {
                BuildSliceScope::Module
            } else {
                BuildSliceScope::Package
            },
            package: "cynic-node".to_string(),
            target: BuildSliceTarget::Lib,
            test_filter: matches!(phase, BuildSlicePhase::Test)
                .then(|| "capability_context".to_string()),
            network_mode: BuildSliceNetworkMode::Offline,
            runner_ref: "role:build_runner".to_string(),
            idempotency_key: "digest:action".to_string(),
            timeout_secs: 300,
            max_jobs: 1,
        }
    }

    #[test]
    fn translates_module_test_to_bounded_argv() {
        assert_eq!(
            build_slice_cargo_argv(&action(BuildSlicePhase::Test)).unwrap(),
            vec![
                "cargo",
                "test",
                "--package",
                "cynic-node",
                "--lib",
                "--offline",
                "--jobs",
                "1",
                "capability_context",
            ]
        );
    }

    #[test]
    fn translates_check_and_build_without_test_filter() {
        for phase in [BuildSlicePhase::Check, BuildSlicePhase::Build] {
            let argv = build_slice_cargo_argv(&action(phase)).unwrap();
            assert!(!argv.iter().any(|argument| argument == "capability_context"));
        }
    }

    #[test]
    fn rejects_non_library_targets_until_target_identity_exists() {
        let mut value = action(BuildSlicePhase::Check);
        value.target = BuildSliceTarget::Test;
        assert!(matches!(
            build_slice_cargo_argv(&value),
            Err(BuildSliceCommandError::UnsupportedTarget(_))
        ));
    }
}
