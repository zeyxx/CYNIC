use super::*;

fn rule(program: &str, prefix: &[&str]) -> AllowRule {
    AllowRule {
        program: program.to_string(),
        arg_prefix: prefix.iter().map(|s| (*s).to_string()).collect(),
        reason: "declared capability".to_string(),
    }
}

#[test]
fn empty_ruleset_refuses_everything() {
    let policy = ExecutionPolicy::default();
    let decision = policy.decide(&["git", "status"], HostRole::Host);
    assert!(!decision.allowed());
    assert_eq!(
        decision,
        ExecutionDecision::Refuse {
            reason: RefusalReason::NotDeclared {
                program: "git".to_string()
            },
            detail: "no rule admits program \"git\"".to_string(),
        }
    );
}

#[test]
fn a_declared_rule_admits() {
    let policy =
        ExecutionPolicy::new(vec![rule("git", &["push"])]).with_build_roles(Vec::new());
    assert!(
        policy
            .decide(&["git", "push", "origin"], HostRole::Host)
            .allowed()
    );
    assert!(!policy.decide(&["git", "pull"], HostRole::Host).allowed());
}

#[test]
fn a_local_build_is_refused_on_a_host_that_may_not_compile() {
    let policy = ExecutionPolicy::new(vec![rule("cargo", &[])]).with_build_roles(Vec::new());
    let decision = policy.decide(&["cargo", "build", "--release"], HostRole::Host);
    assert!(!decision.allowed());
    assert!(matches!(
        decision,
        ExecutionDecision::Refuse {
            reason: RefusalReason::LocalBuildForbidden {
                program: ref program
            },
            ..
        } if program == "cargo"
    ));
}

#[test]
fn a_build_runner_may_compile() {
    let policy = ExecutionPolicy::new(vec![rule("cargo", &[])]);
    assert!(
        policy
            .decide(&["cargo", "check"], HostRole::BuildRunner)
            .allowed()
    );
}

#[test]
fn a_ruleset_cannot_reopen_a_local_build() {
    // The rule admits cargo, but the host role does not allow builds:
    // the structural refusal comes first.
    let policy = ExecutionPolicy::new(vec![rule("cargo", &[])]).with_build_roles(Vec::new());
    let decision = policy.decide(&["cargo", "check"], HostRole::Host);
    assert!(matches!(
        decision,
        ExecutionDecision::Refuse {
            reason: RefusalReason::LocalBuildForbidden { .. },
            ..
        }
    ));
}

#[test]
fn reaching_the_builder_over_ssh_is_refused_from_a_host() {
    let policy = ExecutionPolicy::new(vec![rule("ssh", &[])]).with_build_roles(Vec::new());
    let decision = policy.decide(
        &[
            "ssh",
            "-o",
            "BatchMode=yes",
            "cynic-builder",
            "cargo",
            "build",
        ],
        HostRole::Host,
    );
    assert!(matches!(
        decision,
        ExecutionDecision::Refuse {
            reason: RefusalReason::RemoteBuildRunnerForbidden { .. },
            ..
        }
    ));
}

#[test]
fn reaching_the_builder_over_ssh_is_refused_even_with_a_user_prefix() {
    let policy = ExecutionPolicy::new(vec![rule("ssh", &[])]).with_build_roles(Vec::new());
    let decision = policy.decide(
        &["ssh", "cynic@cynic-builder", "cargo", "build"],
        HostRole::Host,
    );
    assert!(matches!(
        decision,
        ExecutionDecision::Refuse {
            reason: RefusalReason::RemoteBuildRunnerForbidden { .. },
            ..
        }
    ));
}

#[test]
fn reaching_another_host_over_ssh_is_left_to_the_ruleset() {
    let policy = ExecutionPolicy::new(vec![rule("ssh", &[])]).with_build_roles(Vec::new());
    let decision = policy.decide(
        &["ssh", "cynic@cynic-agent-host", "hostname"],
        HostRole::Host,
    );
    assert!(decision.allowed());
}

#[test]
fn a_build_runner_may_reach_itself() {
    let policy = ExecutionPolicy::new(vec![rule("ssh", &[])]);
    let decision = policy.decide(
        &["ssh", "cynic-builder", "cargo", "build"],
        HostRole::BuildRunner,
    );
    assert!(decision.allowed());
}

#[test]
fn the_remote_target_is_found_past_the_options() {
    assert_eq!(
        remote_target(&["ssh", "-o", "BatchMode=yes", "cynic-builder", "true"]),
        Some("cynic-builder".to_string())
    );
    assert_eq!(remote_target(&["scp", "a", "b"]), Some("a".to_string()));
    assert_eq!(remote_target(&["git", "status"]), None);
}

#[test]
fn a_build_program_is_recognised_behind_a_path() {
    assert!(is_build_program("cargo"));
    assert!(is_build_program("/usr/bin/cargo"));
    assert!(!is_build_program("cargo-watch"));
}

#[test]
fn an_unknown_role_cannot_inherit_build_authority() {
    assert_eq!(HostRole::parse("build_runner"), HostRole::BuildRunner);
    assert_eq!(HostRole::parse("cortex_host"), HostRole::Host);
    assert_eq!(HostRole::parse(""), HostRole::Host);
}

#[test]
fn a_catalog_capability_becomes_a_rule() {
    let catalog: crate::privilege::CapabilityCatalog = serde_json::from_str(
        r#"{
          "schema": "cynic.capability_catalog.v1",
          "capabilities": [
            {
              "id": "pct-list",
              "domain": "hypervisor",
              "argv_template": ["/usr/sbin/pct", "list"],
              "params": [],
              "requires_root": true,
              "mandatory": false,
              "purpose": "CT inventory"
            },
            {
              "id": "service-reconcile",
              "domain": "host-service",
              "argv_template": ["/usr/local/bin/cynic", "host", "service", "reconcile", "--node", "{node}"],
              "params": [{"name": "node", "kind": "path_segment"}],
              "requires_root": true,
              "mandatory": false,
              "purpose": "reconcile a systemd unit"
            }
          ]
        }"#,
    )
    .expect("catalog parses");
    let rules = rules_from_catalog(&catalog);
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].program, "/usr/sbin/pct");
    assert_eq!(rules[0].arg_prefix, vec!["list".to_string()]);
    assert_eq!(rules[1].program, "/usr/local/bin/cynic");
    // The prefix carries the literals up to the first placeholder:
    // `--node` is a literal, `{node}` is the unknown value.
    assert_eq!(
        rules[1].arg_prefix,
        vec![
            "host".to_string(),
            "service".to_string(),
            "reconcile".to_string(),
            "--node".to_string(),
        ]
    );
    assert_eq!(rules[1].reason, "capability:service-reconcile");
}

#[test]
fn a_literal_carrying_a_slot_is_not_a_placeholder() {
    assert!(is_placeholder("{node}"));
    assert!(!is_placeholder("--node"));
    assert!(!is_placeholder("--node={node}"));
    assert!(!is_placeholder("{}"));
}

#[test]
fn a_catalog_capability_admits_its_own_argv() {
    let catalog: crate::privilege::CapabilityCatalog = serde_json::from_str(
        r#"{
          "schema": "cynic.capability_catalog.v1",
          "capabilities": [
            {
              "id": "pct-list",
              "domain": "hypervisor",
              "argv_template": ["/usr/sbin/pct", "list"],
              "params": [],
              "requires_root": true,
              "mandatory": false,
              "purpose": "CT inventory"
            }
          ]
        }"#,
    )
    .expect("catalog parses");
    let policy = ExecutionPolicy::new(rules_from_catalog(&catalog));
    assert!(
        policy
            .decide(&["/usr/sbin/pct", "list"], HostRole::Host)
            .allowed()
    );
}

impl ExecutionPolicy {
    fn with_build_roles(mut self, roles: Vec<&str>) -> Self {
        self.build_roles = roles.iter().map(|role| (*role).to_string()).collect();
        self
    }
}
