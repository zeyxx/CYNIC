//! Canonical policy for Python CLI payload provenance and update authority.
//!
//! This keeps observed payload layout separate from the mutator command that
//! should repair or refresh the install.

use super::types::{PythonInstallProvenance, ToolingInstallProvenance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PythonCliPolicy {
    pub dep_id: &'static str,
    pub distribution: &'static str,
    pub expected_tooling_provenance: ToolingInstallProvenance,
    pub expected_python_provenance: PythonInstallProvenance,
    pub canonical_update_argv: &'static [&'static str],
    pub canonical_update_command: &'static str,
    pub canonical_update_commands: &'static [&'static str],
    pub update_authority_note: &'static str,
}

pub fn python_cli_policy(dep_id: &str) -> Option<PythonCliPolicy> {
    match dep_id {
        "hermes-agent" => Some(PythonCliPolicy {
            dep_id: "hermes-agent",
            distribution: "hermes-agent",
            expected_tooling_provenance: ToolingInstallProvenance::UvTool,
            expected_python_provenance: PythonInstallProvenance::UvTool,
            canonical_update_argv: &["uv", "tool", "install", "--force", "hermes-agent"],
            canonical_update_command: "uv tool install --force hermes-agent",
            canonical_update_commands: &[
                "uv tool install --force hermes-agent",
                "hermes --version",
            ],
            update_authority_note: "payload is expected as uv-managed tool; canonical mutator is uv tool install --force",
        }),
        "huggingface-cli" => Some(PythonCliPolicy {
            dep_id: "huggingface-cli",
            distribution: "huggingface_hub",
            expected_tooling_provenance: ToolingInstallProvenance::PipUserSite,
            expected_python_provenance: PythonInstallProvenance::PipUserSite,
            canonical_update_argv: &["hf", "update"],
            canonical_update_command: "hf update",
            canonical_update_commands: &["hf update", "cynic runtime inference-hf-status --json"],
            update_authority_note: "payload is expected as user-site python; hf update selects the mutator path",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::python_cli_policy;

    #[test]
    fn hermes_policy_uses_uv_tool_update_path() {
        let policy =
            python_cli_policy("hermes-agent").expect("hermes-agent policy must be present");
        assert_eq!(
            policy.canonical_update_command,
            "uv tool install --force hermes-agent"
        );
        assert!(
            policy
                .canonical_update_argv
                .iter()
                .any(|arg| *arg == "tool")
                && policy
                    .canonical_update_argv
                    .iter()
                    .any(|arg| *arg == "--force")
        );
    }

    #[test]
    fn unknown_python_cli_policy_is_absent() {
        assert!(python_cli_policy("not-a-dep").is_none());
    }
}
