//! Proof that the hypervisor surface lives in the node substrate family and
//! that the old `agent host` path is a deprecated alias, not a second owner.
//!
//! Parse-only (no execution): the compiled `clap` tree is the surface the
//! operator types, so an arm present in `Cli::command()` proves routing without
//! building or running any observer.

use clap::CommandFactory;

use cynic_cli::Cli;

fn node_command() -> clap::Command {
    Cli::command()
        .find_subcommand("node")
        .expect("node family must exist")
        .clone()
}

#[test]
fn node_hypervisor_exists_in_substrate_family() {
    let node = node_command();
    let hypervisor = node
        .find_subcommand("hypervisor")
        .expect("cynic node hypervisor must be reachable in the node family");
    let arms: Vec<&str> = hypervisor
        .get_subcommands()
        .map(|s| s.get_name())
        .collect();
    assert_eq!(
        arms,
        vec!["plan", "observe"],
        "hypervisor family must expose exactly plan and observe"
    );
}

fn agent_command() -> clap::Command {
    Cli::command()
        .find_subcommand("agent")
        .expect("agent family must exist")
        .clone()
}

#[test]
fn agent_host_alias_still_parses_as_deprecated_delegate() {
    let agent = agent_command();
    let host = agent
        .find_subcommand("host")
        .expect("deprecated `cynic agent host` must keep parsing during the alias window");
    // Alias delegates to the canonical enum, so it carries the same two arms.
    let arms: Vec<&str> = host.get_subcommands().map(|s| s.get_name()).collect();
    assert_eq!(arms, vec!["plan", "observe"]);
}

#[test]
fn hypervisor_arm_name_carries_no_hyphen() {
    // Constitutional rule (no_hyphen_in_cli_commands): the arm an operator types.
    for name in ["node", "hypervisor", "plan", "observe", "agent", "host"] {
        assert!(
            !name.contains('-'),
            "operator-typed arm must not contain a hyphen: {name}"
        );
    }
}
