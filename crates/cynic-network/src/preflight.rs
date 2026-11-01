use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStatus {
    Missing,
    PrincipalMismatch,
    ConnectionRequired,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleDisposition {
    NotDiscovered,
    RequiresPrincipalAdoption,
    RequiresConnection,
    ObservableCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    Observe,
    Adopt,
    Connect,
    RepairService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandObservation {
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TcpObservation {
    pub host: String,
    pub port: u16,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeNetworkPreflightReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: Value,
    pub lifecycle_status: LifecycleStatus,
    pub lifecycle_disposition: LifecycleDisposition,
    pub next_boundary_kind: BoundaryKind,
    pub next_boundary_reason: String,
    pub next_command: Option<String>,
    pub resolver: ResolverBaseline,
    pub probes: Vec<NetworkProbe>,
    pub remediation_plan: NetworkRemediationPlan,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResolverBaseline {
    pub resolv_conf_path: &'static str,
    pub nameservers: Vec<String>,
    pub search_domains: Vec<String>,
    pub uses_local_stub: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NetworkProbe {
    pub host: String,
    pub port: u16,
    pub resolution: CommandObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_ip: Option<String>,
    pub tcp: TcpObservation,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NetworkRemediationPlan {
    pub status: &'static str,
    pub mutates: bool,
    pub activation_surface: &'static str,
    pub required_authority: &'static str,
    pub steps: Vec<NetworkRemediationStep>,
    pub rollback_intent: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NetworkRemediationStep {
    pub id: &'static str,
    pub phase: &'static str,
    pub mutates: bool,
    pub description: &'static str,
    pub proof: &'static str,
}

pub fn parse_resolver_baseline(raw: &str) -> ResolverBaseline {
    let mut nameservers = Vec::new();
    let mut search_domains = Vec::new();

    for line in raw.lines().map(str::trim) {
        if line.starts_with("nameserver ") {
            if let Some(value) = line.split_whitespace().nth(1) {
                nameservers.push(value.to_string());
            }
        } else if line.starts_with("search ") {
            search_domains.extend(line.split_whitespace().skip(1).map(str::to_string));
        }
    }

    let uses_local_stub = nameservers
        .iter()
        .any(|value| value == "127.0.0.53" || value == "::1");

    ResolverBaseline {
        resolv_conf_path: "/etc/resolv.conf",
        nameservers,
        search_domains,
        uses_local_stub,
        read_error: None,
    }
}

pub fn overall_status(resolver: &ResolverBaseline, probes: &[NetworkProbe]) -> &'static str {
    if resolver.read_error.is_some() {
        return "blocked";
    }
    if probes.iter().all(|probe| probe.status == "reachable") {
        "ready"
    } else if probes.iter().any(|probe| probe.status == "blocked") {
        "blocked"
    } else {
        "degraded"
    }
}

pub fn plan_remediation(status: &str) -> NetworkRemediationPlan {
    let steps = if status == "ready" {
        vec![NetworkRemediationStep {
            id: "preserve_observed_resolver",
            phase: "observe",
            mutates: false,
            description: "keep current resolver baseline as evidence for fetch-dependent runtime surfaces",
            proof: "cynic.network.runtime_network_preflight.v1.status=ready",
        }]
    } else {
        vec![
            NetworkRemediationStep {
                id: "classify_resolver_failure",
                phase: "observe",
                mutates: false,
                description: "inspect resolver baseline, failed DNS probes, and TCP reachability before selecting a provider-specific repair",
                proof: "resolver.nameservers + probes[].resolution + probes[].tcp",
            },
            NetworkRemediationStep {
                id: "plan_provider_specific_dns_restore",
                phase: "plan",
                mutates: false,
                description: "select the least-coupled resolver restoration supported by detected host capabilities and runtime layout",
                proof: "planned argv, provider, target files/services, and rollback intent",
            },
            NetworkRemediationStep {
                id: "execute_only_with_explicit_authority",
                phase: "execute",
                mutates: true,
                description: "execute resolver restoration only through a bounded CYNIC host/network activation surface with explicit operator authority",
                proof: "post-execution network preflight report",
            },
        ]
    };

    NetworkRemediationPlan {
        status: if status == "ready" {
            "not_required"
        } else {
            "required"
        },
        mutates: false,
        activation_surface: "cynic runtime resolver-plan --json; read-only upstream-resolver planning surface",
        required_authority: "explicit_host_network_mutation_authority",
        steps,
        rollback_intent: vec![
            "capture resolver files and service state before mutation",
            "restore previous resolver projection if post-execution preflight remains blocked",
            "do not alter operator connectivity from conformance or dev-check surfaces",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_extracts_nameservers_and_search_domains() {
        let baseline = parse_resolver_baseline(
            "# comment\nnameserver 127.0.0.53\noptions edns0 trust-ad\nsearch service.cynic corp\n",
        );

        assert_eq!(baseline.nameservers, vec!["127.0.0.53"]);
        assert_eq!(baseline.search_domains, vec!["service.cynic", "corp"]);
        assert!(baseline.uses_local_stub);
    }

    #[test]
    fn overall_status_ready_when_all_reachable() {
        let resolver = parse_resolver_baseline("nameserver 1.1.1.1\n");
        let probes = vec![NetworkProbe {
            host: "crates.io".to_string(),
            port: 443,
            resolution: CommandObservation {
                command: "getent".to_string(),
                args: vec!["hosts".to_string(), "crates.io".to_string()],
                exit_code: Some(0),
                stdout: "151.101.1.63".to_string(),
                stderr: String::new(),
            },
            resolved_ip: Some("151.101.1.63".to_string()),
            tcp: TcpObservation {
                host: "crates.io".to_string(),
                port: 443,
                status: "open".to_string(),
            },
            status: "reachable",
        }];
        assert_eq!(overall_status(&resolver, &probes), "ready");
    }

    #[test]
    fn plan_remediation_not_required_when_ready() {
        let plan = plan_remediation("ready");
        assert_eq!(plan.status, "not_required");
    }
}