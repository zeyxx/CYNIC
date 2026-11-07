//! Minimal host adapter: `AgentAccessOperation` (L0) -> `HostOperation`.
//!
//! The match is exhaustive with NO wildcard arm: adding a variant to the
//! L0 enum fails compilation here until it is translated. Field shapes
//! are identical by construction in `cynic_protocol::agent_access_port`.
//!
//! Used by `agent_access::{access_apply,authority_apply}` to turn the L2
//! plan's L0 operations into admitted `HostOperation`s.

use cynic_protocol::agent_access_port::AgentAccessOperation;

use crate::ports::HostOperation;

/// Translate one L0 agent-access operation to its host equivalent.
pub fn translate_agent_access_operation(operation: &AgentAccessOperation) -> HostOperation {
    match operation {
        AgentAccessOperation::EnsureSystemGroup { group } => HostOperation::EnsureSystemGroup {
            group: group.clone(),
        },
        AgentAccessOperation::EnsureSupplementaryGroupMember { user, group } => {
            HostOperation::EnsureSupplementaryGroupMember {
                user: user.clone(),
                group: group.clone(),
            }
        }
        AgentAccessOperation::EnsureOwnedDir {
            path,
            user,
            group,
            mode,
        } => HostOperation::EnsureOwnedDir {
            path: path.clone(),
            user: user.clone(),
            group: group.clone(),
            mode: *mode,
        },
        AgentAccessOperation::InstallOwnedFile {
            source,
            target,
            user,
            group,
            mode,
        } => HostOperation::InstallOwnedFile {
            source: source.clone(),
            target: target.clone(),
            user: user.clone(),
            group: group.clone(),
            mode: *mode,
        },
    }
}

/// Translate a batch, preserving order.
pub fn translate_agent_access_operations(
    operations: &[AgentAccessOperation],
) -> Vec<HostOperation> {
    operations
        .iter()
        .map(translate_agent_access_operation)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_four() -> Vec<AgentAccessOperation> {
        vec![
            AgentAccessOperation::EnsureSystemGroup {
                group: "cynic-agent".to_string(),
            },
            AgentAccessOperation::EnsureSupplementaryGroupMember {
                user: "operator".to_string(),
                group: "cynic-agent".to_string(),
            },
            AgentAccessOperation::EnsureOwnedDir {
                path: "/run/cynic".to_string(),
                user: "cynic".to_string(),
                group: "cynic".to_string(),
                mode: 0o755,
            },
            AgentAccessOperation::InstallOwnedFile {
                source: "/src/authority.json".to_string(),
                target: "/etc/cynic/agent-session-authority.json".to_string(),
                user: "root".to_string(),
                group: "cynic".to_string(),
                mode: 0o640,
            },
        ]
    }

    #[test]
    fn translates_each_variant_with_fields_preserved() {
        let translated = translate_agent_access_operations(&all_four());
        assert_eq!(
            translated,
            vec![
                HostOperation::EnsureSystemGroup {
                    group: "cynic-agent".to_string(),
                },
                HostOperation::EnsureSupplementaryGroupMember {
                    user: "operator".to_string(),
                    group: "cynic-agent".to_string(),
                },
                HostOperation::EnsureOwnedDir {
                    path: "/run/cynic".to_string(),
                    user: "cynic".to_string(),
                    group: "cynic".to_string(),
                    mode: 0o755,
                },
                HostOperation::InstallOwnedFile {
                    source: "/src/authority.json".to_string(),
                    target: "/etc/cynic/agent-session-authority.json".to_string(),
                    user: "root".to_string(),
                    group: "cynic".to_string(),
                    mode: 0o640,
                },
            ]
        );
    }

    #[test]
    fn single_translation_matches_batch_and_empty_is_empty() {
        let operation = AgentAccessOperation::EnsureSystemGroup {
            group: "cynic-agent".to_string(),
        };
        assert_eq!(
            translate_agent_access_operation(&operation),
            translate_agent_access_operations(std::slice::from_ref(&operation))[0]
        );
        assert!(translate_agent_access_operations(&[]).is_empty());
    }
}
