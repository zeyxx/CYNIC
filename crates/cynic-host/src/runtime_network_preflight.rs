use serde::Serialize;

use crate::process_observation::{CommandObservation, TcpObservation, resolve_host, tcp_probe};
use crate::runtime_harness::{RuntimeHarnessContext, conformance_context};

const DEFAULT_TARGETS: &[(&str, u16)] = &[("crates.io", 443), ("static.crates.io", 443)];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RuntimeNetworkPreflightReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub context: RuntimeHarnessContext,
    pub resolver: ResolverBaseline,
    pub probes: Vec<NetworkProbe>,
    pub remediation_plan: NetworkRemediationPlan,
    pub rules: Vec<&'static str>,
    pub next_actions: Vec<&'static str>,
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

pub fn observe_runtime_network_preflight(targets: &[String]) -> RuntimeNetworkPreflightReport {
    observe_runtime_network_preflight_with_observer(targets, &LocalRuntimeNetworkObserver)
}

fn observe_runtime_network_preflight_with_observer(
    targets: &[String],
    observer: &impl RuntimeNetworkObservation,
) -> RuntimeNetworkPreflightReport {
    let resolver = observer.observe_resolver_baseline();
    let probes = target_list(targets)
        .into_iter()
        .map(|(host, port)| {
            let resolution = observer.resolve_host(host);
            let resolved_ip = crate::process_observation::resolved_ip_from_command(&resolution);
            let tcp = observer.tcp_probe(host, port);
            let status = if resolution.exit_code == Some(0) && tcp.status != "unresolved" {
                "reachable"
            } else if resolution.exit_code == Some(0) {
                "resolution_only"
            } else {
                "blocked"
            };

            NetworkProbe {
                host: host.to_string(),
                port,
                resolution,
                resolved_ip,
                tcp,
                status,
            }
        })
        .collect::<Vec<_>>();
    let status = overall_status(&resolver, &probes);

    RuntimeNetworkPreflightReport {
        schema: "cynic.host.runtime_network_preflight.v1",
        status,
        mutates: false,
        context: conformance_context(),
        resolver,
        probes,
        remediation_plan: remediation_plan(status),
        rules: vec![
            "network preflight is an observation surface only; it must not mutate host or resolver state",
            "dns failures must be reported explicitly instead of being inferred indirectly from fetch failures",
            "a local resolver stub is not proof of upstream dns health; external resolution still has to be observed",
            "dns remediation must be performed through an explicit network or host activation surface, never implicitly by diagnostics",
        ],
        next_actions: next_actions(status),
    }
}

trait RuntimeNetworkObservation {
    fn observe_resolver_baseline(&self) -> ResolverBaseline;
    fn resolve_host(&self, host: &str) -> CommandObservation;
    fn tcp_probe(&self, host: &str, port: u16) -> TcpObservation;
}

struct LocalRuntimeNetworkObserver;

impl RuntimeNetworkObservation for LocalRuntimeNetworkObserver {
    fn observe_resolver_baseline(&self) -> ResolverBaseline {
        let path = "/etc/resolv.conf";
        match std::fs::read_to_string(path) {
            Ok(raw) => parse_resolver_baseline(&raw),
            Err(error) => ResolverBaseline {
                resolv_conf_path: path,
                nameservers: Vec::new(),
                search_domains: Vec::new(),
                uses_local_stub: false,
                read_error: Some(error.to_string()),
            },
        }
    }

    fn resolve_host(&self, host: &str) -> CommandObservation {
        resolve_host(host)
    }

    fn tcp_probe(&self, host: &str, port: u16) -> TcpObservation {
        tcp_probe(host, port)
    }
}

fn parse_resolver_baseline(raw: &str) -> ResolverBaseline {
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

fn target_list(targets: &[String]) -> Vec<(&str, u16)> {
    if targets.is_empty() {
        return DEFAULT_TARGETS.to_vec();
    }

    targets.iter().map(|host| (host.as_str(), 443)).collect()
}

fn overall_status(resolver: &ResolverBaseline, probes: &[NetworkProbe]) -> &'static str {
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

fn next_actions(status: &str) -> Vec<&'static str> {
    match status {
        "ready" => vec!["network substrate is usable for fetch-dependent runtime surfaces"],
        "degraded" => vec![
            "inspect tcp reachability for partially resolved targets",
            "verify upstream firewall or egress policy before promoting this host as a fetch authority",
        ],
        _ => vec![
            "inspect resolver baseline and failed probes",
            "repair upstream dns or sandbox egress before retrying fetch-dependent runtime surfaces",
            "do not treat package download failures as application bugs until network preflight is ready",
        ],
    }
}

fn remediation_plan(status: &str) -> NetworkRemediationPlan {
    let steps = if status == "ready" {
        vec![NetworkRemediationStep {
            id: "preserve_observed_resolver",
            phase: "observe",
            mutates: false,
            description: "keep current resolver baseline as evidence for fetch-dependent runtime surfaces",
            proof: "cynic.host.runtime_network_preflight.v1.status=ready",
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

    struct FakeObserver {
        resolver: ResolverBaseline,
        probes: Vec<NetworkProbe>,
    }

    impl RuntimeNetworkObservation for FakeObserver {
        fn observe_resolver_baseline(&self) -> ResolverBaseline {
            self.resolver.clone()
        }

        fn resolve_host(&self, host: &str) -> CommandObservation {
            self.probes
                .iter()
                .find(|probe| probe.host == host)
                .expect("probe")
                .resolution
                .clone()
        }

        fn tcp_probe(&self, host: &str, port: u16) -> TcpObservation {
            self.probes
                .iter()
                .find(|probe| probe.host == host && probe.port == port)
                .expect("probe")
                .tcp
                .clone()
        }
    }

    #[test]
    fn reports_ready_when_targets_resolve_and_have_addressability() {
        let observer = FakeObserver {
            resolver: parse_resolver_baseline("nameserver 1.1.1.1\nsearch synapse.cynic\n"),
            probes: vec![probe("crates.io", 443, Some(0), "151.101.1.63", "open")],
        };

        let report =
            observe_runtime_network_preflight_with_observer(&["crates.io".to_string()], &observer);

        assert_eq!(report.status, "ready");
        assert!(!report.mutates);
        assert_eq!(
            report.context.kind,
            crate::runtime_harness::RuntimeHarnessKind::Conformance
        );
    }

    #[test]
    fn reports_blocked_when_resolution_fails_against_local_stub() {
        let observer = FakeObserver {
            resolver: parse_resolver_baseline("nameserver 127.0.0.53\nsearch .\n"),
            probes: vec![probe("static.crates.io", 443, Some(2), "", "unresolved")],
        };

        let report = observe_runtime_network_preflight_with_observer(
            &["static.crates.io".to_string()],
            &observer,
        );

        assert_eq!(report.status, "blocked");
        assert!(report.resolver.uses_local_stub);
        assert_eq!(report.probes[0].status, "blocked");
    }

    #[test]
    fn parser_extracts_nameservers_and_search_domains() {
        let baseline = parse_resolver_baseline(
            "# comment\nnameserver 127.0.0.53\noptions edns0 trust-ad\nsearch synapse.cynic corp\n",
        );

        assert_eq!(baseline.nameservers, vec!["127.0.0.53"]);
        assert_eq!(baseline.search_domains, vec!["synapse.cynic", "corp"]);
        assert!(baseline.uses_local_stub);
    }

    fn probe(
        host: &str,
        port: u16,
        exit_code: Option<i32>,
        stdout: &str,
        tcp_status: &str,
    ) -> NetworkProbe {
        let status = if exit_code == Some(0) && tcp_status != "unresolved" {
            "reachable"
        } else if exit_code == Some(0) {
            "resolution_only"
        } else {
            "blocked"
        };

        NetworkProbe {
            host: host.to_string(),
            port,
            resolution: CommandObservation {
                command: "getent".to_string(),
                args: vec!["hosts".to_string(), host.to_string()],
                exit_code,
                stdout: stdout.to_string(),
                stderr: String::new(),
            },
            resolved_ip: if stdout.is_empty() {
                None
            } else {
                Some(stdout.to_string())
            },
            tcp: TcpObservation {
                host: host.to_string(),
                port,
                status: tcp_status.to_string(),
            },
            status,
        }
    }
}
