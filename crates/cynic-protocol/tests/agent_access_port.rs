//! Phase Extraire (Cycle 2.2, Verticale 3/3) : le port L0
//! `AgentAccessOperation` + traits Observation/Execution existe en
//! `cynic_protocol::agent_access_port`.
//!
//! Ce test échoue (erreur de compilation) tant que le port n'a pas été
//! créé en L0. Il borne aussi le périmètre : exactement les 4 variantes
//! de `HostOperation` consommées par `agent_access`
//! (`decision.rs` + `authority_projection.rs`), rien d'autre.

use cynic_protocol::agent_access_port::{
    AgentAccessExecutionPort, AgentAccessObservationPort, AgentAccessOperation,
};

#[test]
fn agent_access_operation_has_exactly_the_four_consumed_variants() {
    let operations = [
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
    ];

    // Exhaustive match WITHOUT wildcard: adding a 5th variant to the
    // enum breaks this test at compile time until it is classified.
    let kinds: Vec<&str> = operations
        .iter()
        .map(|operation| match operation {
            AgentAccessOperation::EnsureSystemGroup { .. } => "ensure_system_group",
            AgentAccessOperation::EnsureSupplementaryGroupMember { .. } => {
                "ensure_supplementary_group_member"
            }
            AgentAccessOperation::EnsureOwnedDir { .. } => "ensure_owned_dir",
            AgentAccessOperation::InstallOwnedFile { .. } => "install_owned_file",
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "ensure_system_group",
            "ensure_supplementary_group_member",
            "ensure_owned_dir",
            "install_owned_file",
        ]
    );

    // Field-level round-trip through the exhaustive match.
    for operation in &operations {
        match operation {
            AgentAccessOperation::EnsureSystemGroup { group } => {
                assert_eq!(group, "cynic-agent");
            }
            AgentAccessOperation::EnsureSupplementaryGroupMember { user, group } => {
                assert_eq!((user.as_str(), group.as_str()), ("operator", "cynic-agent"));
            }
            AgentAccessOperation::EnsureOwnedDir {
                path,
                user,
                group,
                mode,
            } => {
                assert_eq!(
                    (path.as_str(), user.as_str(), group.as_str(), *mode),
                    ("/run/cynic", "cynic", "cynic", 0o755)
                );
            }
            AgentAccessOperation::InstallOwnedFile {
                source,
                target,
                user,
                group,
                mode,
            } => {
                assert_eq!(
                    (
                        source.as_str(),
                        target.as_str(),
                        user.as_str(),
                        group.as_str(),
                        *mode
                    ),
                    (
                        "/src/authority.json",
                        "/etc/cynic/agent-session-authority.json",
                        "root",
                        "cynic",
                        0o640
                    )
                );
            }
        }
    }
}

struct FakeAgentAccessPorts;

impl AgentAccessObservationPort<String> for FakeAgentAccessPorts {
    fn observe_agent_access(
        &self,
        access_group: &str,
        runtime_directory: &str,
        socket_path: &str,
    ) -> Result<String, String> {
        Ok(format!(
            "observed:{access_group}:{runtime_directory}:{socket_path}"
        ))
    }
}

impl AgentAccessExecutionPort for FakeAgentAccessPorts {
    fn execute_agent_access_operation(
        &self,
        operation: &AgentAccessOperation,
    ) -> Result<(), String> {
        match operation {
            AgentAccessOperation::EnsureSystemGroup { group } => {
                assert_eq!(group, "cynic-agent");
                Ok(())
            }
            AgentAccessOperation::EnsureSupplementaryGroupMember { .. }
            | AgentAccessOperation::EnsureOwnedDir { .. }
            | AgentAccessOperation::InstallOwnedFile { .. } => Ok(()),
        }
    }
}

#[test]
fn observation_and_execution_traits_are_usable_from_l0() {
    let ports = FakeAgentAccessPorts;
    assert_eq!(
        ports
            .observe_agent_access("cynic-agent", "/run/cynic", "/run/cynic/kernel.sock")
            .unwrap(),
        "observed:cynic-agent:/run/cynic:/run/cynic/kernel.sock"
    );
    ports
        .execute_agent_access_operation(&AgentAccessOperation::EnsureSystemGroup {
            group: "cynic-agent".to_string(),
        })
        .unwrap();
}
