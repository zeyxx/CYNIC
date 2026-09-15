//! Node-owned translation boundary for typed Builder slice actions.
//!
//! This module validates an action and translates it to a bounded Cargo argv.
//! It does not spawn processes, choose a runner, or grant execution authority.

use cynic_build::build_slice::{
    BuildSliceAction, BuildSliceNetworkMode, BuildSlicePhase, BuildSliceTarget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildSliceCommandError {
    InvalidAction(Vec<String>),
    UnsupportedTarget(String),
}

impl std::fmt::Display for BuildSliceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAction(violations) => {
                write!(
                    formatter,
                    "invalid build slice action: {}",
                    violations.join(",")
                )
            }
            Self::UnsupportedTarget(target) => {
                write!(formatter, "build slice target not supported yet: {target}")
            }
        }
    }
}

impl std::error::Error for BuildSliceCommandError {}

pub fn build_slice_cargo_argv(
    action: &BuildSliceAction,
) -> Result<Vec<String>, BuildSliceCommandError> {
    let violations = action.validate();
    if !violations.is_empty() {
        return Err(BuildSliceCommandError::InvalidAction(violations));
    }
    if action.target != BuildSliceTarget::Lib {
        return Err(BuildSliceCommandError::UnsupportedTarget(format!(
            "{:?}",
            action.target
        )));
    }

    let mut argv = vec!["cargo".to_string()];
    match action.phase {
        BuildSlicePhase::Check => argv.push("check".to_string()),
        BuildSlicePhase::Test => argv.push("test".to_string()),
        BuildSlicePhase::Build => argv.push("build".to_string()),
    }
    argv.extend([
        "--package".to_string(),
        action.package.clone(),
        "--lib".to_string(),
    ]);
    if matches!(action.network_mode, BuildSliceNetworkMode::Offline) {
        argv.push("--offline".to_string());
    }
    argv.extend(["--jobs".to_string(), action.max_jobs.to_string()]);
    if let Some(filter) = &action.test_filter {
        argv.push(filter.clone());
    }
    Ok(argv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_build::build_slice::{BUILD_SLICE_ACTION_SCHEMA, BuildSliceScope};

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
