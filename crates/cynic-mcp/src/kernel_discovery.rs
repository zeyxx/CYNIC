use cynic_host::runtime_registry::resolve_runtime_registry_path;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::net::Ipv6Addr;
use std::time::Duration;

const DEFAULT_PROBE_TIMEOUT_MS: u64 = 1_500;
const MIN_PROBE_TIMEOUT_MS: u64 = 100;
const MAX_PROBE_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
struct KernelCandidate {
    url: String,
    node_id: Option<String>,
    node_name: Option<String>,
    role: String,
    status: Option<String>,
    deploy_target: bool,
    aliases: Vec<String>,
    source: String,
}

impl KernelCandidate {
    fn label(&self) -> String {
        self.node_id
            .as_deref()
            .or(self.node_name.as_deref())
            .unwrap_or(self.url.as_str())
            .to_string()
    }
}

#[derive(Debug)]
struct ProbeObservation {
    candidate: KernelCandidate,
    verified: bool,
    detail: String,
    identity: Option<KernelIdentity>,
}

#[derive(Debug, Clone, Deserialize)]
struct KernelBuildIdentity {
    version: String,
    git_sha: String,
    profile: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KernelIdentity {
    schema: String,
    service: String,
    node_id: String,
    hostname: String,
    pid: u32,
    build: KernelBuildIdentity,
}

#[derive(Debug)]
pub(crate) struct ResolvedKernelEndpoint {
    pub url: String,
    pub node_id: Option<String>,
    pub node_name: Option<String>,
    pub role: String,
    pub status: Option<String>,
    pub source: String,
    pub proof: String,
    pub hostname: String,
    pub pid: u32,
    pub version: String,
    pub git_sha: String,
    pub build_profile: String,
}

pub(crate) async fn resolve_kernel_endpoint(
    api_key: &str,
) -> Result<ResolvedKernelEndpoint, String> {
    let timeout = probe_timeout();
    let client = reqwest::Client::builder()
        .connect_timeout(timeout)
        .timeout(timeout)
        .build()
        .map_err(|error| format!("cannot build kernel discovery client: {error}"))?;

    if let Ok(raw_addr) = std::env::var("CYNIC_REST_ADDR") {
        let candidate = KernelCandidate {
            url: normalize_base_url(&raw_addr)?,
            node_id: None,
            node_name: None,
            role: "explicit".to_string(),
            status: None,
            deploy_target: false,
            aliases: Vec::new(),
            source: "env:CYNIC_REST_ADDR".to_string(),
        };
        let observation = probe_candidate(&client, candidate, api_key).await;
        return select_verified_kernel(vec![observation]);
    }

    let mut candidates = registry_kernel_candidates()?;
    append_seed_candidates(&mut candidates)?;
    let mut observations = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        observations.push(probe_candidate(&client, candidate, api_key).await);
    }
    select_verified_kernel(observations)
}

fn append_seed_candidates(candidates: &mut Vec<KernelCandidate>) -> Result<(), String> {
    let mut seen: HashSet<String> = candidates
        .iter()
        .map(|candidate| candidate.url.clone())
        .collect();
    let Some(raw_seeds) = std::env::var("CYNIC_KERNEL_SEEDS").ok() else {
        return Ok(());
    };
    for raw_seed in raw_seeds
        .split(',')
        .map(str::trim)
        .filter(|seed| !seed.is_empty())
    {
        let url = normalize_base_url(raw_seed)?;
        if !seen.insert(url.clone()) {
            continue;
        }
        candidates.push(KernelCandidate {
            url,
            node_id: None,
            node_name: None,
            role: "discovery_seed".to_string(),
            status: None,
            deploy_target: false,
            aliases: Vec::new(),
            source: "env:CYNIC_KERNEL_SEEDS".to_string(),
        });
    }
    Ok(())
}

fn probe_timeout() -> Duration {
    let milliseconds = std::env::var("CYNIC_KERNEL_DISCOVERY_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PROBE_TIMEOUT_MS)
        .clamp(MIN_PROBE_TIMEOUT_MS, MAX_PROBE_TIMEOUT_MS);
    Duration::from_millis(milliseconds)
}

fn registry_kernel_candidates() -> Result<Vec<KernelCandidate>, String> {
    let registry_path = resolve_runtime_registry_path(None)?;
    let content = std::fs::read_to_string(&registry_path)
        .map_err(|error| format!("cannot read {}: {error}", registry_path.display()))?;
    let registry: Value = serde_json::from_str(&content)
        .map_err(|error| format!("cannot parse {}: {error}", registry_path.display()))?;
    candidates_from_registry(&registry, &registry_path.display().to_string())
}

fn candidates_from_registry(
    registry: &Value,
    source_path: &str,
) -> Result<Vec<KernelCandidate>, String> {
    let deploy_role = registry
        .pointer("/lifecycle/kernel_release/deploy_target_role")
        .and_then(Value::as_str)
        .unwrap_or("kernel_primary");
    let nodes = registry
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{source_path} has no nodes array"))?;

    let mut candidates = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut rejected = Vec::new();

    for node in nodes
        .iter()
        .filter(|node| kernel_service_port(node).is_some())
    {
        let label = node
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| node.get("name").and_then(Value::as_str))
            .unwrap_or("<unnamed-kernel-node>");
        let Some(host) = node.pointer("/network/synapse").and_then(Value::as_str) else {
            rejected.push(format!("{label}: missing network.synapse"));
            continue;
        };
        let Some(port) = kernel_service_port(node) else {
            rejected.push(format!("{label}: missing valid cynic-kernel service port"));
            continue;
        };
        let raw_url = format!("http://{}:{port}", format_host(host));
        let url = normalize_base_url(&raw_url)
            .map_err(|error| format!("invalid kernel endpoint for {label}: {error}"))?;
        if !seen_urls.insert(url.clone()) {
            rejected.push(format!("{label}: duplicate endpoint {url}"));
            continue;
        }

        candidates.push(KernelCandidate {
            url,
            node_id: node.get("id").and_then(Value::as_str).map(String::from),
            node_name: node.get("name").and_then(Value::as_str).map(String::from),
            role: node
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or(deploy_role)
                .to_string(),
            status: node.get("status").and_then(Value::as_str).map(String::from),
            deploy_target: node
                .get("deploy_target")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            aliases: node
                .get("aliases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect(),
            source: format!("registry:{source_path}"),
        });
    }

    if candidates.is_empty() {
        let suffix = if rejected.is_empty() {
            "no matching nodes".to_string()
        } else {
            rejected.join(", ")
        };
        return Err(format!(
            "no usable kernel candidates for role={deploy_role} in {source_path}: {suffix}"
        ));
    }
    Ok(candidates)
}

fn kernel_service_port(node: &Value) -> Option<u16> {
    node.get("services")
        .and_then(Value::as_array)
        .and_then(|services| {
            services
                .iter()
                .find(|service| service.get("name").and_then(Value::as_str) == Some("cynic-kernel"))
        })
        .and_then(|service| service.get("port"))
        .and_then(Value::as_u64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port != 0)
}

fn format_host(host: &str) -> String {
    if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn normalize_base_url(raw_addr: &str) -> Result<String, String> {
    let raw_addr = raw_addr.trim();
    if raw_addr.is_empty() {
        return Err("kernel address is empty".to_string());
    }
    let with_scheme = if raw_addr.starts_with("http://") || raw_addr.starts_with("https://") {
        raw_addr.to_string()
    } else {
        format!("http://{raw_addr}")
    };
    let parsed = reqwest::Url::parse(&with_scheme)
        .map_err(|error| format!("invalid kernel URL {with_scheme:?}: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("unsupported kernel URL scheme {}", parsed.scheme()));
    }
    if parsed.host_str().is_none() || parsed.port_or_known_default().is_none() {
        return Err(format!("kernel URL has no host or port: {with_scheme}"));
    }
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(
            "kernel address must be an origin without path, query, or fragment".to_string(),
        );
    }
    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

async fn probe_candidate(
    client: &reqwest::Client,
    candidate: KernelCandidate,
    api_key: &str,
) -> ProbeObservation {
    let endpoint = format!("{}/identity", candidate.url);
    let mut request = client.get(&endpoint);
    if !api_key.is_empty() {
        request = request.bearer_auth(api_key);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return ProbeObservation {
                candidate,
                verified: false,
                detail: format!("unreachable: {error}"),
                identity: None,
            };
        }
    };
    let status = response.status();
    if !status.is_success() {
        return ProbeObservation {
            candidate,
            verified: false,
            detail: format!("identity probe returned HTTP {status}"),
            identity: None,
        };
    }
    let identity = match response.json::<KernelIdentity>().await {
        Ok(identity) => identity,
        Err(error) => {
            return ProbeObservation {
                candidate,
                verified: false,
                detail: format!("identity probe returned invalid JSON: {error}"),
                identity: None,
            };
        }
    };
    let contract_matches = identity.schema == "cynic.kernel.identity.v1"
        && identity.service == "cynic-kernel"
        && !identity.build.version.is_empty()
        && !identity.build.git_sha.is_empty();
    let identity_matches = candidate.node_id.is_none()
        || candidate
            .node_id
            .iter()
            .chain(candidate.node_name.iter())
            .chain(candidate.aliases.iter())
            .any(|declared| declared == &identity.node_id || declared == &identity.hostname);
    let verified = contract_matches && identity_matches;
    ProbeObservation {
        candidate,
        verified,
        detail: if verified {
            format!(
                "GET /identity matched node={} version={} git_sha={}",
                identity.node_id, identity.build.version, identity.build.git_sha
            )
        } else if contract_matches {
            format!(
                "kernel identity node={} hostname={} does not match declared registry node",
                identity.node_id, identity.hostname
            )
        } else {
            "identity probe did not match the CYNIC kernel identity contract".to_string()
        },
        identity: Some(identity),
    }
}

fn select_verified_kernel(
    observations: Vec<ProbeObservation>,
) -> Result<ResolvedKernelEndpoint, String> {
    let verified: Vec<&ProbeObservation> = observations
        .iter()
        .filter(|observation| observation.verified)
        .collect();
    if verified.len() > 1 {
        let identities = verified
            .iter()
            .map(|observation| {
                format!(
                    "{}={}",
                    observation.candidate.label(),
                    observation.candidate.url
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "kernel discovery is ambiguous; multiple verified kernels are reachable: {identities}"
        ));
    }
    if let Some(observation) = verified.first() {
        let candidate = &observation.candidate;
        let identity = observation
            .identity
            .as_ref()
            .ok_or_else(|| "verified kernel has no identity evidence".to_string())?;
        return Ok(ResolvedKernelEndpoint {
            url: candidate.url.clone(),
            node_id: Some(identity.node_id.clone()),
            node_name: candidate.node_name.clone(),
            role: candidate.role.clone(),
            status: candidate.status.clone(),
            source: format!("{}+observed", candidate.source),
            proof: observation.detail.clone(),
            hostname: identity.hostname.clone(),
            pid: identity.pid,
            version: identity.build.version.clone(),
            git_sha: identity.build.git_sha.clone(),
            build_profile: identity.build.profile.clone(),
        });
    }

    let failures = observations
        .iter()
        .map(|observation| {
            format!(
                "{}={} ({})",
                observation.candidate.label(),
                observation.candidate.url,
                observation.detail
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    Err(format!(
        "no CYNIC kernel could be observed; candidates are declarations only: {failures}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_fixture() -> Value {
        serde_json::json!({
            "lifecycle": {"kernel_release": {"deploy_target_role": "kernel_primary"}},
            "nodes": [
                {
                    "id": "kernel-a",
                    "name": "core-a",
                    "role": "kernel_primary",
                    "status": "candidate",
                    "deploy_target": true,
                    "aliases": ["kernel-a.internal"],
                    "network": {"synapse": "10.10.10.10"},
                    "services": [{"name": "cynic-kernel", "port": 3030}]
                },
                {
                    "id": "builder-a",
                    "role": "build_runner",
                    "network": {"synapse": "10.10.10.11"},
                    "services": []
                }
            ]
        })
    }

    fn identity(node_id: &str, hostname: &str, git_sha: &str) -> KernelIdentity {
        KernelIdentity {
            schema: "cynic.kernel.identity.v1".to_string(),
            service: "cynic-kernel".to_string(),
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            pid: 42,
            build: KernelBuildIdentity {
                version: "v1".to_string(),
                git_sha: git_sha.to_string(),
                profile: "release".to_string(),
            },
        }
    }

    #[test]
    fn registry_entries_are_candidates_not_runtime_truth() {
        let candidates = candidates_from_registry(&registry_fixture(), "fixture").unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].node_id.as_deref(), Some("kernel-a"));
        assert_eq!(candidates[0].url, "http://10.10.10.10:3030");
        assert_eq!(candidates[0].status.as_deref(), Some("candidate"));
    }

    #[test]
    fn exactly_one_observed_kernel_is_required() {
        let candidate = candidates_from_registry(&registry_fixture(), "fixture")
            .unwrap()
            .remove(0);
        let resolved = select_verified_kernel(vec![ProbeObservation {
            candidate,
            verified: true,
            detail: "identity matched".to_string(),
            identity: Some(identity("kernel-a", "core-a", "abc")),
        }])
        .unwrap();
        assert_eq!(resolved.node_id.as_deref(), Some("kernel-a"));
        assert!(resolved.source.ends_with("+observed"));
    }

    #[test]
    fn ambiguous_observed_kernels_are_rejected() {
        let candidate = candidates_from_registry(&registry_fixture(), "fixture")
            .unwrap()
            .remove(0);
        let second = KernelCandidate {
            url: "http://10.10.10.12:3030".to_string(),
            node_id: Some("kernel-b".to_string()),
            aliases: Vec::new(),
            ..candidate.clone()
        };
        let error = select_verified_kernel(vec![
            ProbeObservation {
                candidate,
                verified: true,
                detail: "identity matched".to_string(),
                identity: Some(identity("kernel-a", "core-a", "abc")),
            },
            ProbeObservation {
                candidate: second,
                verified: true,
                detail: "identity matched".to_string(),
                identity: Some(identity("kernel-b", "core-b", "def")),
            },
        ])
        .unwrap_err();
        assert!(error.contains("ambiguous"));
    }

    #[test]
    fn unobserved_candidate_is_rejected() {
        let candidate = candidates_from_registry(&registry_fixture(), "fixture")
            .unwrap()
            .remove(0);
        let error = select_verified_kernel(vec![ProbeObservation {
            candidate,
            verified: false,
            detail: "unreachable".to_string(),
            identity: None,
        }])
        .unwrap_err();
        assert!(error.contains("declarations only"));
    }

    #[test]
    fn normalizes_ipv4_dns_and_ipv6_origins() {
        assert_eq!(
            normalize_base_url("kernel.test:3030").unwrap(),
            "http://kernel.test:3030"
        );
        assert_eq!(
            normalize_base_url("http://[fd00::10]:3030").unwrap(),
            "http://[fd00::10]:3030"
        );
        assert!(normalize_base_url("http://kernel.test:3030/health").is_err());
    }
}
