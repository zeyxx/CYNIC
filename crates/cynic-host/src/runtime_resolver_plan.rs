use serde::Serialize;

use crate::runtime_harness::{RuntimeHarnessContext, conformance_context};
use crate::runtime_network_preflight::{
    RuntimeNetworkPreflightReport, observe_runtime_network_preflight,
};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeResolverPlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: RuntimeHarnessContext,
    pub authority_owner: &'static str,
    pub desired_state: ResolverDesiredState,
    pub preflight: RuntimeNetworkPreflightReport,
    pub phases: Vec<ResolverPlanPhase>,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolverDesiredState {
    pub scope: &'static str,
    pub resolver_role: &'static str,
    pub service_contract: &'static str,
    pub must_not_own: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolverPlanPhase {
    pub id: &'static str,
    pub phase: &'static str,
    pub mutates: bool,
    pub required_authority: &'static str,
    pub description: &'static str,
    pub proof: &'static str,
}

pub fn build_runtime_resolver_plan(targets: &[String]) -> RuntimeResolverPlanReport {
    let preflight = observe_runtime_network_preflight(targets);
    let status = if preflight.status == "ready" {
        "not_required"
    } else {
        "planned"
    };

    RuntimeResolverPlanReport {
        schema: "cynic.host.runtime_resolver_plan.v1",
        status,
        mutates: false,
        context: conformance_context(),
        authority_owner: "cynic-host",
        desired_state: ResolverDesiredState {
            scope: "host upstream resolver for external package, registry, and artifact reachability",
            resolver_role: "restore external DNS and TCP reachability without owning internal service DNS",
            service_contract: "fetch-dependent runtime surfaces can resolve and connect to declared external targets",
            must_not_own: vec![
                "synapse.cynic internal DNS zone",
                "CoreDNS bundle rendering",
                "mesh provider selection",
                "implicit host network mutation from diagnostics",
            ],
        },
        preflight,
        phases: resolver_plan_phases(),
        rules: vec![
            "resolver planning belongs to cynic-host because it qualifies host connectivity",
            "internal service DNS remains owned by cynic-node",
            "this plan is read-only and must not mutate resolver files or services",
            "a future execution surface must capture before-state, exact argv, post-state, and rollback proof",
        ],
        next_actions: next_actions(status),
    }
}

fn resolver_plan_phases() -> Vec<ResolverPlanPhase> {
    vec![
        ResolverPlanPhase {
            id: "observe_current_resolver",
            phase: "observe",
            mutates: false,
            required_authority: "none",
            description: "capture /etc/resolv.conf baseline, nameservers, search domains, and external target probes",
            proof: "preflight.resolver + preflight.probes",
        },
        ResolverPlanPhase {
            id: "classify_failure_domain",
            phase: "plan",
            mutates: false,
            required_authority: "none",
            description: "distinguish resolver failure, TCP egress failure, sandbox network denial, and package registry outage",
            proof: "preflight.status + probes[].resolution.exit_code + probes[].tcp.status",
        },
        ResolverPlanPhase {
            id: "select_host_resolver_adapter",
            phase: "plan",
            mutates: false,
            required_authority: "host_network_planning_authority",
            description: "select a distro/provider adapter only after detected host capabilities prove it is supported",
            proof: "declared adapter, target files, target services, and rollback intent",
        },
        ResolverPlanPhase {
            id: "execute_with_operator_authority",
            phase: "execute",
            mutates: true,
            required_authority: "explicit_host_network_mutation_authority",
            description: "perform resolver restoration only through a bounded execution surface, never from diagnostics",
            proof: "before-state, exact argv, exit codes, after-state, and repeated runtime network preflight",
        },
    ]
}

fn next_actions(status: &str) -> Vec<&'static str> {
    if status == "not_required" {
        return vec!["preserve current resolver baseline as fetch-readiness evidence"];
    }

    vec![
        "inspect preflight resolver and probe failure details",
        "choose or implement the smallest host resolver adapter for the observed service manager and distro",
        "do not route this through cynic-node internal DNS planning",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolver_plan_is_read_only_and_owned_by_host() {
        let report = build_runtime_resolver_plan(&["example.invalid".to_string()]);

        assert_eq!(report.schema, "cynic.host.runtime_resolver_plan.v1");
        assert_eq!(report.authority_owner, "cynic-host");
        assert!(!report.mutates);
        assert!(
            report
                .desired_state
                .must_not_own
                .contains(&"synapse.cynic internal DNS zone")
        );
        assert!(report.phases.iter().any(|phase| phase.phase == "execute"));
        assert!(
            report
                .rules
                .contains(&"internal service DNS remains owned by cynic-node")
        );
    }
}
