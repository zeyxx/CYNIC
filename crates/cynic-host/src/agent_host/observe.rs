//! Proxmox preflight observation adapter for Agent Host VM.
//!
//! Design rules enforced here:
//! - No endpoint, token identity, node name, storage pool, template name, or
//!   bridge is ever hardcoded. Every input comes from `AgentHostDesired` (the
//!   declared desired state) or from dynamic provider observation.
//! - TLS is never fail-open. The client is built with an explicit CA when
//!   `ca_pem` is supplied; without it the observer returns `blocked_tls_unproven`.
//! - The Proxmox node name is resolved dynamically from `/api2/json/nodes`
//!   matched against `desired.hypervisor_target`. Ambiguous or missing →
//!   `unknown` / `blocked`.
//! - Template presence is checked against `desired.template_spec` (exact volid
//!   match). No substring guessing.
//! - Storage content is queried on `desired.storage_pool`, not on `local`.

use cynic_protocol::AgentHostDesired;
use serde_json::Value;

use cynic_lifecycle::ArgvRunner;

use super::types::{AGENT_HOST_PREFLIGHT_REPORT_SCHEMA, AgentHostPreflightReport};

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

/// Port for observing hypervisor preflight conditions.
pub trait AgentHostObservationPort {
    fn observe_agent_host_preflight(
        &self,
        desired: &AgentHostDesired,
    ) -> Result<AgentHostPreflightReport, String>;
}

// ---------------------------------------------------------------------------
// Node resolution result
// ---------------------------------------------------------------------------

enum ResolvedNode {
    /// Exactly one node matched `hypervisor_target`.
    Ok(String),
    /// Zero or >1 nodes matched — cannot proceed safely.
    Ambiguous(String),
    /// API unreachable or nodes list empty.
    Unknown(String),
}

impl ResolvedNode {
    fn as_blocker(&self) -> Option<String> {
        match self {
            ResolvedNode::Ok(_) => None,
            ResolvedNode::Ambiguous(msg) | ResolvedNode::Unknown(msg) => Some(msg.clone()),
        }
    }

    fn node_name(&self) -> Option<&str> {
        if let ResolvedNode::Ok(n) = self {
            Some(n.as_str())
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// REST observer
// ---------------------------------------------------------------------------

/// Canonical Proxmox hypervisor observer via direct HTTPS REST API (Port 8006).
///
/// Constructed via `ProxmoxRestAgentHostObserver::new`; callers must supply
/// the endpoint, token identity, token secret, and an optional CA PEM bundle.
/// No defaults, no env reads — those belong in the CLI thin projection.
pub struct ProxmoxRestAgentHostObserver {
    base_url: String,
    token_id: String,
    token_secret: String,
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for ProxmoxRestAgentHostObserver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxmoxRestAgentHostObserver")
            .field("base_url", &self.base_url)
            .field("token_id", &self.token_id)
            .finish_non_exhaustive()
    }
}

impl ProxmoxRestAgentHostObserver {
    /// Build an observer with explicit TLS policy.
    ///
    /// `ca_pem`: PEM-encoded CA certificate bytes. If `None`, the client
    /// refuses to build (`blocked_tls_unproven`) — fail-closed by design.
    pub fn new(
        base_url: impl Into<String>,
        token_id: impl Into<String>,
        token_secret: impl Into<String>,
        ca_pem: Option<&[u8]>,
    ) -> Result<Self, String> {
        let Some(pem) = ca_pem else {
            return Err("blocked_tls_unproven:no_ca_pem_supplied".to_string());
        };
        let cert = reqwest::blocking::Client::builder().build().ok(); // just to validate; real cert below
        drop(cert);
        let certificate = reqwest::Certificate::from_pem(pem)
            .map_err(|e| format!("blocked_tls_invalid_ca:{e}"))?;
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .add_root_certificate(certificate)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;

        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token_id: token_id.into(),
            token_secret: token_secret.into(),
            client,
        })
    }

    fn auth_header(&self) -> String {
        format!("PVEAPIToken={}={}", self.token_id, self.token_secret)
    }

    fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let res = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .map_err(|e| format!("pve_api_request_failed:{e}"))?;

        if !res.status().is_success() {
            return Err(format!("pve_api_http_error:{}", res.status()));
        }

        let json: Value = res.json().map_err(|e| format!("invalid_pve_json:{e}"))?;
        Ok(json.get("data").cloned().unwrap_or(json))
    }

    /// Discover and resolve the Proxmox node name from `desired.hypervisor_target`.
    ///
    /// Queries `/api2/json/nodes`, matches the `node` field against
    /// `hypervisor_target`. Returns `Ambiguous` when >1 match, `Unknown`
    /// when unreachable or empty.
    fn resolve_node(&self, hypervisor_target: &str) -> ResolvedNode {
        match self.get_json("/api2/json/nodes") {
            Err(e) => ResolvedNode::Unknown(format!("hypervisor_nodes_unreachable:{e}")),
            Ok(nodes) => {
                let Some(arr) = nodes.as_array() else {
                    return ResolvedNode::Unknown(
                        "hypervisor_nodes_response_not_array".to_string(),
                    );
                };
                let matches: Vec<String> = arr
                    .iter()
                    .filter_map(|n| n.get("node").and_then(Value::as_str))
                    .filter(|name| *name == hypervisor_target)
                    .map(String::from)
                    .collect();
                match matches.len() {
                    0 => ResolvedNode::Ambiguous(format!(
                        "hypervisor_node_not_found:target={hypervisor_target}"
                    )),
                    1 => ResolvedNode::Ok(matches.into_iter().next().unwrap()),
                    n => ResolvedNode::Ambiguous(format!(
                        "hypervisor_node_ambiguous:target={hypervisor_target},matches={n}"
                    )),
                }
            }
        }
    }
}

impl AgentHostObservationPort for ProxmoxRestAgentHostObserver {
    fn observe_agent_host_preflight(
        &self,
        desired: &AgentHostDesired,
    ) -> Result<AgentHostPreflightReport, String> {
        let mut blockers = Vec::new();

        // 0. Resolve Proxmox node name from desired.hypervisor_target
        let resolved_node = self.resolve_node(&desired.hypervisor_target);
        if let Some(b) = resolved_node.as_blocker() {
            blockers.push(b);
        }

        // 1. Node status — only if node resolved
        let (
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
        ) = if let Some(node) = resolved_node.node_name() {
            match self.get_json(&format!("/api2/json/nodes/{node}/status")) {
                Ok(parsed) => {
                    let version = parsed["pveversion"].as_str().map(ToString::to_string);
                    let mem_total = parsed["memory"]["total"].as_u64().unwrap_or(0);
                    let mem_used = parsed["memory"]["used"].as_u64().unwrap_or(0);
                    let mem_avail = mem_total.saturating_sub(mem_used);
                    let disk_avail = parsed["rootfs"]["avail"].as_u64().unwrap_or(0);
                    (true, version, mem_avail, disk_avail)
                }
                Err(e) => {
                    blockers.push(format!("hypervisor_api_unreachable:{e}"));
                    (false, None, 0, 0)
                }
            }
        } else {
            (false, None, 0, 0)
        };

        // 2. Cluster resources — VMID + declared storage pool
        let mut vmid_available = true;
        let mut vmid_existing_status = None;
        let mut storage_pool_available = false;

        if let Ok(resources) = self.get_json("/api2/json/cluster/resources") {
            if let Some(arr) = resources.as_array() {
                for item in arr {
                    if let Some(vmid) = item.get("vmid").and_then(Value::as_u64) {
                        if vmid == desired.vmid as u64 {
                            vmid_available = false;
                            vmid_existing_status = item
                                .get("status")
                                .and_then(Value::as_str)
                                .map(ToString::to_string);
                        }
                    }
                    if item.get("type").and_then(Value::as_str) == Some("storage")
                        && item.get("storage").and_then(Value::as_str)
                            == Some(&desired.storage_pool)
                    {
                        storage_pool_available = true;
                    }
                }
            }
        }

        if !storage_pool_available {
            blockers.push(format!(
                "storage_pool_not_found:pool={}",
                desired.storage_pool
            ));
        }

        // 3. Template — exact volid match against desired.template_spec
        //    Queried on the declared storage_pool, not hardcoded "local".
        let mut template_present = false;
        let mut template_volid = None;

        match (&resolved_node.node_name(), &desired.template_spec) {
            (Some(node), Some(spec)) => {
                let path = format!(
                    "/api2/json/nodes/{node}/storage/{}/content",
                    desired.storage_pool
                );
                match self.get_json(&path) {
                    Ok(contents) => {
                        if let Some(arr) = contents.as_array() {
                            for item in arr {
                                if let Some(volid) = item.get("volid").and_then(Value::as_str) {
                                    if volid == spec.as_str() {
                                        template_present = true;
                                        template_volid = Some(volid.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                        if !template_present {
                            blockers.push(format!(
                                "template_not_found:spec={spec},pool={}",
                                desired.storage_pool
                            ));
                        }
                    }
                    Err(e) => {
                        blockers.push(format!("template_storage_unobservable:{e}"));
                    }
                }
            }
            (None, _) => {
                blockers.push("template_check_skipped:node_unresolved".to_string());
            }
            (_, None) => {
                blockers.push("template_spec_undeclared:blocked".to_string());
            }
        }

        // 4. Network bridge — from declared desired.network_bridge
        let mut network_bridge_present = false;
        if let Some(node) = resolved_node.node_name() {
            if let Ok(ifaces) = self.get_json(&format!("/api2/json/nodes/{node}/network")) {
                if let Some(arr) = ifaces.as_array() {
                    for item in arr {
                        if item.get("iface").and_then(Value::as_str)
                            == Some(&desired.network_bridge)
                        {
                            network_bridge_present = true;
                            break;
                        }
                    }
                }
            }
        }

        Ok(AgentHostPreflightReport {
            schema: AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
            vmid_available,
            vmid_existing_status,
            template_present,
            template_volid,
            storage_pool_available,
            network_bridge_present,
            blockers,
        })
    }
}

// ---------------------------------------------------------------------------
// SSH fallback observer
// ---------------------------------------------------------------------------

/// Proxmox hypervisor observer via SSH runner executing pvesh.
///
/// Used as a breakglass fallback when REST is unavailable. Applies the same
/// dynamic node resolution as the REST observer — no `/nodes/pve/` literal.
pub struct ProxmoxSshAgentHostObserver<'a, R: ArgvRunner> {
    runner: &'a R,
    ssh_target: String,
}

impl<'a, R: ArgvRunner> ProxmoxSshAgentHostObserver<'a, R> {
    pub fn new(runner: &'a R, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_proxmox_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn pvesh(&self, args: &[String]) -> Result<cynic_lifecycle::ArgvOutput, String> {
        let mut argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pvesh".to_string(),
        ];
        argv.extend(args.iter().cloned());
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if output.success() {
            Ok(output)
        } else {
            Err(format!(
                "proxmox_pvesh_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ))
        }
    }

    /// Resolve Proxmox node name from `hypervisor_target` via `/nodes` listing.
    fn resolve_node(&self, hypervisor_target: &str) -> ResolvedNode {
        let res = self.pvesh(&[
            "get".to_string(),
            "/nodes".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);
        match res {
            Err(e) => ResolvedNode::Unknown(format!("hypervisor_nodes_unreachable:{e}")),
            Ok(output) => match serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                Err(e) => ResolvedNode::Unknown(format!("hypervisor_nodes_parse_failed:{e}")),
                Ok(nodes) => {
                    let matches: Vec<String> = nodes
                        .iter()
                        .filter_map(|n| n.get("node").and_then(Value::as_str))
                        .filter(|name| *name == hypervisor_target)
                        .map(String::from)
                        .collect();
                    match matches.len() {
                        0 => ResolvedNode::Ambiguous(format!(
                            "hypervisor_node_not_found:target={hypervisor_target}"
                        )),
                        1 => ResolvedNode::Ok(matches.into_iter().next().unwrap()),
                        n => ResolvedNode::Ambiguous(format!(
                            "hypervisor_node_ambiguous:target={hypervisor_target},matches={n}"
                        )),
                    }
                }
            },
        }
    }
}

impl<R: ArgvRunner> AgentHostObservationPort for ProxmoxSshAgentHostObserver<'_, R> {
    fn observe_agent_host_preflight(
        &self,
        desired: &AgentHostDesired,
    ) -> Result<AgentHostPreflightReport, String> {
        let mut blockers = Vec::new();

        // 0. Resolve node dynamically — same logic as REST observer
        let resolved_node = self.resolve_node(&desired.hypervisor_target);
        if let Some(b) = resolved_node.as_blocker() {
            blockers.push(b);
        }

        // 1. Node status
        let (
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
        ) = if let Some(node) = resolved_node.node_name() {
            let status_res = self.pvesh(&[
                "get".to_string(),
                format!("/nodes/{node}/status"),
                "--output-format".to_string(),
                "json".to_string(),
            ]);
            match status_res {
                Ok(output) => match serde_json::from_str::<Value>(output.stdout.trim()) {
                    Ok(parsed) => {
                        let version = parsed["pveversion"].as_str().map(ToString::to_string);
                        let mem_total = parsed["memory"]["total"].as_u64().unwrap_or(0);
                        let mem_used = parsed["memory"]["used"].as_u64().unwrap_or(0);
                        let mem_avail = mem_total.saturating_sub(mem_used);
                        let disk_avail = parsed["rootfs"]["avail"].as_u64().unwrap_or(0);
                        (true, version, mem_avail, disk_avail)
                    }
                    Err(e) => {
                        blockers.push(format!("hypervisor_status_parse_failed:{e}"));
                        (false, None, 0, 0)
                    }
                },
                Err(e) => {
                    blockers.push(format!("hypervisor_status_unreachable:{e}"));
                    (false, None, 0, 0)
                }
            }
        } else {
            (false, None, 0, 0)
        };

        // 2. Cluster resources — VMID + declared storage pool
        let mut vmid_available = true;
        let mut vmid_existing_status = None;
        let mut storage_pool_available = false;

        let resources_res = self.pvesh(&[
            "get".to_string(),
            "/cluster/resources".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ]);

        if let Ok(output) = resources_res {
            if let Ok(resources) = serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                for item in resources {
                    if let Some(vmid) = item.get("vmid").and_then(Value::as_u64) {
                        if vmid == desired.vmid as u64 {
                            vmid_available = false;
                            vmid_existing_status = item
                                .get("status")
                                .and_then(Value::as_str)
                                .map(ToString::to_string);
                        }
                    }
                    if item.get("type").and_then(Value::as_str) == Some("storage")
                        && item.get("storage").and_then(Value::as_str)
                            == Some(&desired.storage_pool)
                    {
                        storage_pool_available = true;
                    }
                }
            }
        }

        if !storage_pool_available {
            blockers.push(format!(
                "storage_pool_not_found:pool={}",
                desired.storage_pool
            ));
        }

        // 3. Template — exact volid match on declared storage_pool
        let mut template_present = false;
        let mut template_volid = None;

        match (resolved_node.node_name(), &desired.template_spec) {
            (Some(node), Some(spec)) => {
                let content_res = self.pvesh(&[
                    "get".to_string(),
                    format!("/nodes/{node}/storage/{}/content", desired.storage_pool),
                    "--output-format".to_string(),
                    "json".to_string(),
                ]);
                match content_res {
                    Ok(output) => {
                        if let Ok(contents) =
                            serde_json::from_str::<Vec<Value>>(output.stdout.trim())
                        {
                            for item in contents {
                                if let Some(volid) = item.get("volid").and_then(Value::as_str) {
                                    if volid == spec.as_str() {
                                        template_present = true;
                                        template_volid = Some(volid.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                        if !template_present {
                            blockers.push(format!(
                                "template_not_found:spec={spec},pool={}",
                                desired.storage_pool
                            ));
                        }
                    }
                    Err(e) => {
                        blockers.push(format!("template_storage_unobservable:{e}"));
                    }
                }
            }
            (None, _) => {
                blockers.push("template_check_skipped:node_unresolved".to_string());
            }
            (_, None) => {
                blockers.push("template_spec_undeclared:blocked".to_string());
            }
        }

        // 4. Network bridge — from declared desired.network_bridge
        let mut network_bridge_present = false;
        if let Some(node) = resolved_node.node_name() {
            let network_res = self.pvesh(&[
                "get".to_string(),
                format!("/nodes/{node}/network"),
                "--output-format".to_string(),
                "json".to_string(),
            ]);
            if let Ok(output) = network_res {
                if let Ok(ifaces) = serde_json::from_str::<Vec<Value>>(output.stdout.trim()) {
                    for item in ifaces {
                        if item.get("iface").and_then(Value::as_str)
                            == Some(&desired.network_bridge)
                        {
                            network_bridge_present = true;
                            break;
                        }
                    }
                }
            }
        }

        Ok(AgentHostPreflightReport {
            schema: AGENT_HOST_PREFLIGHT_REPORT_SCHEMA.to_string(),
            hypervisor_reachable,
            hypervisor_version,
            available_memory_bytes,
            available_disk_bytes,
            vmid_available,
            vmid_existing_status,
            template_present,
            template_volid,
            storage_pool_available,
            network_bridge_present,
            blockers,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::{ArgvOutput, ArgvRunner};
    use cynic_protocol::{AgentHostDesired, agent_host_sandbox::AgentHostHarness};

    // ---- Mock runner -------------------------------------------------------

    struct MockRunner {
        nodes_json: String,
        status_json: String,
        resources_json: String,
        storage_content_json: String,
        network_json: String,
        fail_nodes: bool,
    }

    impl MockRunner {
        fn happy(node: &str, storage_pool: &str, template_volid: &str) -> Self {
            Self {
                nodes_json: format!(r#"[{{"node":"{node}","status":"online"}}]"#),
                status_json: r#"{"pveversion":"8.2","memory":{"total":8000000000,"used":2000000000},"rootfs":{"avail":20000000000}}"#.to_string(),
                resources_json: format!(
                    r#"[{{"type":"storage","storage":"{storage_pool}","node":"{node}"}}]"#
                ),
                storage_content_json: format!(
                    r#"[{{"volid":"{template_volid}","content":"vztmpl"}}]"#
                ),
                network_json: r#"[{"iface":"vmbr0","type":"bridge"}]"#.to_string(),
                fail_nodes: false,
            }
        }

        fn unreachable_nodes() -> Self {
            Self {
                fail_nodes: true,
                nodes_json: String::new(),
                status_json: String::new(),
                resources_json: String::new(),
                storage_content_json: String::new(),
                network_json: String::new(),
            }
        }
    }

    impl ArgvRunner for MockRunner {
        fn run(&self, argv: &[&str]) -> ArgvOutput {
            if self.fail_nodes && argv.contains(&"/nodes") {
                return ArgvOutput {
                    stdout: String::new(),
                    stderr: "connection refused".to_string(),
                    exit_code: Some(1),
                    spawn_error: Some("connection refused".to_string()),
                };
            }
            let body = if argv.contains(&"/nodes") || argv.iter().any(|a| *a == "/nodes") {
                &self.nodes_json
            } else if argv.iter().any(|a| a.contains("/status")) {
                &self.status_json
            } else if argv.iter().any(|a| a.contains("/cluster/resources")) {
                &self.resources_json
            } else if argv.iter().any(|a| a.contains("/content")) {
                &self.storage_content_json
            } else if argv.iter().any(|a| a.contains("/network")) {
                &self.network_json
            } else {
                &self.nodes_json
            };
            ArgvOutput {
                stdout: body.clone(),
                stderr: String::new(),
                exit_code: Some(0),
                spawn_error: None,
            }
        }
    }

    fn desired(
        hypervisor_target: &str,
        storage_pool: &str,
        template_spec: Option<&str>,
    ) -> AgentHostDesired {
        AgentHostDesired {
            hypervisor_target: hypervisor_target.to_string(),
            storage_pool: storage_pool.to_string(),
            template_spec: template_spec.map(ToString::to_string),
            ..AgentHostDesired::default()
        }
    }

    // ---- Node resolution tests --------------------------------------------

    #[test]
    fn node_resolved_happy_path() {
        let runner = MockRunner::happy("pve", "local-lvm", "local-lvm:vztmpl/ubuntu.tar.zst");
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        let d = desired("pve", "local-lvm", Some("local-lvm:vztmpl/ubuntu.tar.zst"));
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(report.hypervisor_reachable);
        assert!(report.storage_pool_available);
        assert!(report.template_present);
        assert!(
            report.blockers.is_empty(),
            "unexpected blockers: {:?}",
            report.blockers
        );
    }

    #[test]
    fn node_not_found_is_blocked() {
        let runner = MockRunner::happy("pve", "local-lvm", "local-lvm:vztmpl/ubuntu.tar.zst");
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        // hypervisor_target does not match the mock node "pve"
        let d = desired(
            "other-node",
            "local-lvm",
            Some("local-lvm:vztmpl/ubuntu.tar.zst"),
        );
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(!report.hypervisor_reachable);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.contains("hypervisor_node_not_found")),
            "expected node_not_found blocker, got: {:?}",
            report.blockers
        );
    }

    #[test]
    fn node_unreachable_is_unknown() {
        let runner = MockRunner::unreachable_nodes();
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        let d = desired("pve", "local-lvm", Some("local-lvm:vztmpl/ubuntu.tar.zst"));
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(!report.hypervisor_reachable);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.contains("hypervisor_nodes_unreachable")),
            "expected nodes_unreachable blocker, got: {:?}",
            report.blockers
        );
    }

    #[test]
    fn pool_absent_is_blocked() {
        let runner = MockRunner::happy("pve", "local-lvm", "local-lvm:vztmpl/ubuntu.tar.zst");
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        // Request a pool that does not exist in the mock
        let d = desired(
            "pve",
            "nonexistent-pool",
            Some("local-lvm:vztmpl/ubuntu.tar.zst"),
        );
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(!report.storage_pool_available);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.contains("storage_pool_not_found")),
            "expected storage_pool_not_found blocker, got: {:?}",
            report.blockers
        );
    }

    #[test]
    fn template_absent_is_blocked() {
        let runner = MockRunner::happy("pve", "local-lvm", "local-lvm:vztmpl/ubuntu.tar.zst");
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        // Declare a spec that does not match the mock volid
        let d = desired("pve", "local-lvm", Some("local-lvm:vztmpl/WRONG.tar.zst"));
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(!report.template_present);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.contains("template_not_found")),
            "expected template_not_found blocker, got: {:?}",
            report.blockers
        );
    }

    #[test]
    fn template_spec_undeclared_is_blocked() {
        let runner = MockRunner::happy("pve", "local-lvm", "local-lvm:vztmpl/ubuntu.tar.zst");
        let obs = ProxmoxSshAgentHostObserver::new(&runner, "root@proxmox-forge").unwrap();
        let d = desired("pve", "local-lvm", None);
        let report = obs.observe_agent_host_preflight(&d).unwrap();
        assert!(!report.template_present);
        assert!(
            report
                .blockers
                .iter()
                .any(|b| b.contains("template_spec_undeclared")),
            "expected template_spec_undeclared blocker, got: {:?}",
            report.blockers
        );
    }

    #[test]
    fn rest_observer_blocked_without_ca() {
        let result = ProxmoxRestAgentHostObserver::new(
            "https://pve.synapse.cynic:8006",
            "cynic@pve!runtime",
            "secret",
            None, // no CA → must fail closed
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("blocked_tls_unproven"),
            "expected tls_unproven error, got: {err}"
        );
    }
}
