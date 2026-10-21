//! Deny-by-default execution policy for process spawning.
//!
//! WHY THIS EXISTS.
//!
//! The argv allowlist (`argv_allow.rs`) names a verdict but does not
//! enforce it: against an empty ruleset every argv is `WouldDeny` and
//! still executes. That is an observation log, not a membrane. An agent
//! that holds a shell can therefore reach a build runner with
//! `ssh builder cargo build` and compile on a host that is not allowed to
//! compile, or bypass a declared capability entirely.
//!
//! This module is the decision that closes that gap. It is pure: it takes
//! an argv, a declared policy, and the calling host's role, and returns
//! allow or refuse. No I/O, no spawning, no reading of the host.
//!
//! THE DOCTRINE.
//!
//! Deny by default: an argv is refused unless a declared rule admits it.
//! Two refusals are structural, not configurable:
//!
//! 1. A local build is never admitted. The build runs on the builder, and
//!    the builder is reached through a declared transport capability. A
//!    host that is not a build runner must not compile, whatever the
//!    ruleset says — otherwise the ruleset becomes a second, weaker
//!    authority.
//! 2. Reaching a build runner over `ssh` is not admitted from a host
//!    that is not itself a build surface. The build surface is the
//!    declared capability; an ad-hoc `ssh` line to the builder is the
//!    bypass this module exists to refuse.
//!
//! The ruleset is DATA, not code: it is the capability catalogue, loaded
//! by the caller. This module never reads a file, so the policy can be
//! reviewed as a diff and promoted without touching code.

use serde::{Deserialize, Serialize};

use crate::argv_allow::{AllowRule, ArgvVerdict, observe_argv_verdict};

/// Schema of the deny-by-default execution policy.
pub const EXECUTION_POLICY_SCHEMA: &str = "cynic.execution_policy.v1";

/// Programs that are build tools. A local invocation of one of these is
/// a local build, whatever the arguments are: `cargo check` and
/// `cargo build --release` are the same act for this policy.
pub const BUILD_PROGRAMS: &[&str] = &["cargo", "rustc", "rustup", "cargo-make", "make"];

/// Programs that reach another host.
pub const REMOTE_EXEC_PROGRAMS: &[&str] = &["ssh", "scp", "rsync", "sftp"];

/// The role of the host asking for an execution. It decides whether the
/// host may run a build at all, and whether it may reach a build runner
/// over the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostRole {
    /// A host that may compile: the builder.
    BuildRunner,
    /// An ordinary host: a node, an operator workstation, an agent host.
    Host,
}

/// Why an execution was refused. The reasons are named so a report can
/// distinguish "not declared" from "declared but forbidden here".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum RefusalReason {
    /// The argv is not admitted by any declared rule.
    NotDeclared { program: String },
    /// A local build was attempted from a host that may not compile.
    LocalBuildForbidden { program: String },
    /// A build runner was reached over the network from a host that is
    /// not a build surface.
    RemoteBuildRunnerForbidden { program: String, target: String },
}

/// The decision for one execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "verdict")]
pub enum ExecutionDecision {
    /// The argv may run.
    Allow { rule: String },
    /// The argv must not run.
    Refuse {
        reason: RefusalReason,
        /// The human-readable explanation, for the report.
        detail: String,
    },
}

impl ExecutionDecision {
    pub fn allowed(&self) -> bool {
        matches!(self, ExecutionDecision::Allow { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            ExecutionDecision::Allow { .. } => "allow",
            ExecutionDecision::Refuse { .. } => "refuse",
        }
    }
}

/// The declared policy: the rules that admit an argv, plus the roles that
/// are allowed to act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub schema: String,
    /// The ruleset, as admitted by the capability catalogue.
    #[serde(default)]
    pub rules: Vec<AllowRule>,
    /// The host roles that may compile. A host whose role is not listed
    /// is refused every local build.
    #[serde(default)]
    pub build_roles: Vec<String>,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            schema: EXECUTION_POLICY_SCHEMA.to_string(),
            rules: Vec::new(),
            build_roles: vec![HostRole::BuildRunner.as_str().to_string()],
        }
    }
}

impl HostRole {
    pub fn as_str(self) -> &'static str {
        match self {
            HostRole::BuildRunner => "build_runner",
            HostRole::Host => "host",
        }
    }

    /// Parse a role name as declared in the fleet registry. Unknown
    /// names are refused as `Host`: a role that is not understood must
    /// not inherit build authority.
    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "build_runner" => HostRole::BuildRunner,
            _ => HostRole::Host,
        }
    }
}

impl ExecutionPolicy {
    pub fn new(rules: Vec<AllowRule>) -> Self {
        Self {
            rules,
            ..Self::default()
        }
    }

    /// True when the role may compile.
    pub fn may_build(&self, role: HostRole) -> bool {
        self.build_roles
            .iter()
            .any(|candidate| candidate == role.as_str())
    }

    /// The pure decision.
    ///
    /// Order matters: the structural refusals are evaluated first, so a
    /// ruleset that happens to admit `cargo` cannot re-open a local build
    /// on a host that may not compile. The ruleset decides what else may
    /// run; it never decides that a build may happen here.
    pub fn decide(&self, argv: &[&str], role: HostRole) -> ExecutionDecision {
        let Some(program) = argv.first().copied() else {
            return ExecutionDecision::Refuse {
                reason: RefusalReason::NotDeclared {
                    program: String::new(),
                },
                detail: "empty argv".to_string(),
            };
        };
        let base = program.rsplit('/').next().unwrap_or(program);

        if is_build_program(base) && !self.may_build(role) {
            return ExecutionDecision::Refuse {
                reason: RefusalReason::LocalBuildForbidden {
                    program: base.to_string(),
                },
                detail: format!(
                    "{base} is a build tool and the host role {:?} may not compile; \
                     the build runs on the builder through a declared capability",
                    role.as_str()
                ),
            };
        }

        if is_remote_exec_program(base) {
            if let Some(target) = remote_target(argv) {
                if self.reaches_build_runner(&target) && role != HostRole::BuildRunner {
                    return ExecutionDecision::Refuse {
                        reason: RefusalReason::RemoteBuildRunnerForbidden {
                            program: base.to_string(),
                            target: target.clone(),
                        },
                        detail: format!(
                            "{base} toward the build runner {target:?} is refused for a \
                             non-build host; the build surface is the declared capability"
                        ),
                    };
                }
            }
        }

        match observe_argv_verdict(argv, &self.rules) {
            ArgvVerdict::Admitted { rule } => ExecutionDecision::Allow { rule },
            ArgvVerdict::WouldDeny { reason } => ExecutionDecision::Refuse {
                reason: RefusalReason::NotDeclared {
                    program: base.to_string(),
                },
                detail: reason,
            },
        }
    }

    /// True when the remote target names a build runner. The target is
    /// matched on its host part, so `user@builder` and `builder` are the
    /// same host.
    fn reaches_build_runner(&self, target: &str) -> bool {
        let host = target.rsplit('@').next().unwrap_or(target);
        let host = host.split(':').next().unwrap_or(host);
        BUILD_RUNNER_TARGETS.iter().any(|candidate| {
            *candidate == host
                || host
                    == candidate
                        .strip_suffix(".service.cynic")
                        .unwrap_or(candidate)
        })
    }
}

/// The hosts that are build runners. A target naming one of them over
/// the network is refused from a host that is not a build surface.
pub const BUILD_RUNNER_TARGETS: &[&str] = &[
    "cynic-builder",
    "cynic-builder.service.cynic",
    "cynic-builder-build",
];

/// True when the program is a build tool.
pub fn is_build_program(program: &str) -> bool {
    BUILD_PROGRAMS.contains(&program)
}

/// True when the program reaches another host.
pub fn is_remote_exec_program(program: &str) -> bool {
    REMOTE_EXEC_PROGRAMS.contains(&program)
}

/// The remote target of an argv, when the program reaches another host.
/// The target is the first argument that looks like a host: it is either
/// `user@host` or a bare host, and it never starts with a dash.
pub fn remote_target(argv: &[&str]) -> Option<String> {
    let program = argv.first().copied()?;
    if !is_remote_exec_program(program.rsplit('/').next().unwrap_or(program)) {
        return None;
    }
    for arg in argv.iter().skip(1) {
        if arg.starts_with('-') {
            // An option may carry its value in the next argument, but a
            // host never starts with a dash: skip the option and any
            // value that follows it only when the option takes one.
            continue;
        }
        if *arg == "host" || *arg == "hostname" {
            continue;
        }
        return Some((*arg).to_string());
    }
    None
}


/// Build the execution ruleset from a declared capability catalog.
///
/// The catalog is the single authority for what may run: a capability
/// declares the exact argv it composes, so the rule that admits it is
/// the leading literal arguments of that template, up to the first
/// placeholder. A template with no placeholder admits the whole argv;
/// a template whose first element is a placeholder admits the program
/// alone, because nothing more is known statically.
///
/// The rules are pure data: a policy change is a catalogue diff, and
/// the diff is reviewed like any other declaration.
pub fn rules_from_catalog(
    catalog: &crate::privilege::CapabilityCatalog,
) -> Vec<AllowRule> {
    catalog
        .capabilities
        .iter()
        .map(|capability| {
            let prefix: Vec<String> = capability
                .argv_template
                .iter()
                .take_while(|part| !part.contains('{'))
                .cloned()
                .collect();
            AllowRule {
                program: capability
                    .argv_template
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                arg_prefix: prefix.into_iter().skip(1).collect(),
                reason: format!("capability:{}", capability.id),
            }
        })
        .filter(|rule| !rule.program.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
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
        assert_eq!(
            rules[1].arg_prefix,
            vec!["host".to_string(), "service".to_string(), "reconcile".to_string()]
        );
        assert_eq!(rules[1].reason, "capability:service-reconcile");
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
}
