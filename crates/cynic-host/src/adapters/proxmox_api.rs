//! Proxmox VE REST adapter — same NodeSubstrate ports as the SSH/pvesh
//! adapter, but speaks HTTPS directly with a scoped API token
//! (`provider_control.token_identity` + secret from `credential_ref`).
//!
//! Anti-responsibilities honored (fragment provider_substrate_control.json):
//! - CLI/MCP never own credentials: the token is injected, read host-side.
//! - No raw shell strings: exec takes argv slices only.
//! - Unknown provider state stays an explicit observation (unknowns vec).

use cynic_protocol::{
    NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateAction,
    NodeSubstrateDesiredState, NodeSubstrateExistence, NodeSubstrateObservation,
    NodeSubstratePower,
};

use crate::ports::{NodeSubstrateControlPort, NodeSubstrateObservationPort};

/// Injected HTTP transport (production = ureq/reqwest; tests = mock).
pub trait HttpTransport {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Result<(u16, String), String>;
}

#[derive(Debug, Clone)]
pub struct PveToken {
    pub identity: String,
    pub secret: String,
}

impl PveToken {
    pub fn header_value(&self) -> String {
        format!("PVEAPIToken={}={}", self.identity, self.secret)
    }
}

pub struct ProxmoxApiSubstrateAdapter<T: HttpTransport> {
    pub transport: T,
    pub endpoint_origin: String,
    pub pve_node: String,
    pub token: PveToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PveTaskState {
    Ok { exitcode: i64 },
    Failed { exitcode: Option<i64> },
    Running,
    Unknown(String),
}

impl PveTaskState {
    pub fn parse(status: &str, exitcode: Option<i64>) -> Self {
        match status {
            "ok" => PveTaskState::Ok { exitcode: exitcode.unwrap_or(0) },
            "stopped" => PveTaskState::Failed { exitcode },
            "running" => PveTaskState::Running,
            other => PveTaskState::Unknown(other.to_string()),
        }
    }
    pub fn succeeded(&self) -> bool {
        matches!(self, PveTaskState::Ok { exitcode } if *exitcode == 0)
    }
}

impl<T: HttpTransport> ProxmoxApiSubstrateAdapter<T> {
    pub fn new(transport: T, endpoint_origin: &str, pve_node: &str, token: PveToken) -> Self {
        Self {
            transport,
            endpoint_origin: endpoint_origin.trim_end_matches('/').to_string(),
            pve_node: pve_node.to_string(),
            token,
        }
    }

    fn resource<'b>(
        &self,
        desired: &'b NodeSubstrateDesiredState,
    ) -> Result<(&'b str, u32), String> {
        if desired.provider.as_deref() != Some("proxmox") {
            return Err("unsupported_node_substrate_provider".into());
        }
        let resource = desired
            .resource_id
            .as_deref()
            .ok_or_else(|| "provider_resource_ref_missing".to_string())?;
        let (kind, id) = resource
            .split_once('/')
            .ok_or_else(|| "invalid_proxmox_resource_ref".to_string())?;
        if kind != "lxc"
            || id.is_empty()
            || !id.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err("invalid_proxmox_resource_ref".into());
        }
        let vmid: u32 = id.parse().map_err(|_| "invalid_proxmox_resource_ref")?;
        Ok((kind, vmid))
    }

    fn get_json(&self, path: &str) -> Result<serde_json::Value, String> {
        let (status, body) = self
            .transport
            .request("GET", &format!("/api2/json{path}"), None)?;
        if status != 200 {
            return Err(format!("pve_http_{status}:{}", truncate(&body)));
        }
        serde_json::from_str(&body).map_err(|e| format!("pve_bad_json:{e}"))
    }

    /// POST an LXC exec task; returns the UPID.
    pub fn lxc_exec_start(&self, vmid: u32, command_argv: &[&str]) -> Result<String, String> {
        // PVE form expects repeated `command` entries; we encode argv safely.
        let mut body = String::from("vmid=");
        body.push_str(&vmid.to_string());
        for arg in command_argv {
            body.push_str("&command=");
            body.push_str(&urlencode(arg));
        }
        let (status, resp_body) = self.transport.request(
            "POST",
            &format!("/api2/json/nodes/{}/lxc/{vmid}/exec", self.pve_node),
            Some(&body),
        )?;
        if !(200..300).contains(&status) {
            return Err(format!("pve_exec_http_{status}:{}", truncate(&resp_body)));
        }
        let v: serde_json::Value =
            serde_json::from_str(&resp_body).map_err(|e| format!("pve_bad_json:{e}"))?;
        v.get("data")
            .and_then(|d| d.as_str())
            .map(String::from)
            .ok_or_else(|| "pve_exec_no_upid".to_string())
    }

    /// Poll a UPID task until terminal or bounded timeout.
    pub fn task_wait(
        &self,
        upid: &str,
        max_polls: u32,
        sleep_ms_between: u64,
    ) -> Result<PveTaskState, String> {
        let parts: Vec<&str> = upid.split(':').collect();
        if parts.len() < 6 || parts[0] != "UPID" {
            return Err(format!("pve_bad_upid:{upid}"));
        }
        let node = parts[1];
        let path = format!(
            "/api2/json/nodes/{node}/tasks/{}/status",
            urlencode(upid)
        );
        for _ in 0..max_polls {
            let v = self.get_json(&path)?;
            let status = v
                .pointer("/data/status")
                .and_then(|s| s.as_str())
                .unwrap_or("");
            let exitcode = v.pointer("/data/exitcode").and_then(|x| x.as_i64());
            let st = PveTaskState::parse(status, exitcode);
            match st {
                PveTaskState::Running => {
                    std::thread::sleep(std::time::Duration::from_millis(sleep_ms_between));
                    continue;
                }
                terminal => return Ok(terminal),
            }
        }
        Err(format!("pve_task_timeout_after_{max_polls}_polls"))
    }

    fn lxc_config_raw(&self, vmid: u32) -> Result<serde_json::Value, String> {
        self.get_json(&format!("/nodes/{}/lxc/{vmid}/config", self.pve_node))
    }
}

fn truncate(s: &str) -> String {
    s.chars().take(160).collect()
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

impl<T: HttpTransport> NodeSubstrateObservationPort for ProxmoxApiSubstrateAdapter<T> {
    fn observe_node_substrate(
        &self,
        desired: &NodeSubstrateDesiredState,
    ) -> Result<NodeSubstrateObservation, String> {
        let (_kind, vmid) = self.resource(desired)?;
        let status_path = format!("/nodes/{}/lxc/{vmid}/status/current", self.pve_node);
        let config_path = format!("/nodes/{}/lxc/{vmid}/config", self.pve_node);
        let status = self.get_json(&status_path)?;
        let config = self.lxc_config_raw(vmid)?;

        let power = match status.pointer("/data/status").and_then(|s| s.as_str()) {
            Some("running") => NodeSubstratePower::Running,
            Some("stopped") => NodeSubstratePower::Stopped,
            _ => NodeSubstratePower::Unknown,
        };
        let network = config.pointer("/data/net0").and_then(|v| v.as_str());
        let network_attachment = match network {
            Some(value) if value.contains("bridge=") => NodeNetworkAttachment::Attached,
            Some(_) => NodeNetworkAttachment::Detached,
            None => NodeNetworkAttachment::Unknown,
        };
        let observed_addresses = network
            .and_then(|value| value.split(',').find_map(|f| f.strip_prefix("ip=")))
            .filter(|address| *address != "dhcp")
            .map(|address| vec![address.to_string()])
            .unwrap_or_default();
        Ok(NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: desired.node_id.clone(),
            provider: desired.provider.clone(),
            resource_id: desired.resource_id.clone(),
            existence: NodeSubstrateExistence::Present,
            power,
            network_attachment,
            observed_addresses,
            management_reachable: None,
            observation_authority: Some(format!(
                "proxmox:api:{}@{}",
                self.token.identity, self.endpoint_origin
            )),
            observed_at_unix_ms: Some(current_unix_ms()),
            evidence_refs: vec![
                format!("proxmox:{status_path}"),
                format!("proxmox:{config_path}"),
            ],
            unknowns: if network.is_some_and(|value| value.contains("ip=dhcp")) {
                vec!["dhcp_lease_not_observed_by_provider_config".to_string()]
            } else {
                Vec::new()
            },
        })
    }
}

impl<T: HttpTransport> NodeSubstrateControlPort for ProxmoxApiSubstrateAdapter<T> {
    fn apply_node_substrate_action(
        &self,
        desired: &NodeSubstrateDesiredState,
        action: &NodeSubstrateAction,
    ) -> Result<(), String> {
        let (_kind, vmid) = self.resource(desired)?;
        match action {
            NodeSubstrateAction::Start | NodeSubstrateAction::Stop => {
                let operation = if matches!(action, NodeSubstrateAction::Start) {
                    "start"
                } else {
                    "stop"
                };
                let (status, body) = self.transport.request(
                    "POST",
                    &format!(
                        "/api2/json/nodes/{}/lxc/{vmid}/status/{operation}",
                        self.pve_node
                    ),
                    None,
                )?;
                if !(200..300).contains(&status) {
                    return Err(format!("pve_power_http_{status}:{}", truncate(&body)));
                }
                Ok(())
            }
            NodeSubstrateAction::Provision => Err(
                "provision_requires_principal_plan_use_provision_node_principal".to_string(),
            ),
            _ => Err("unsupported_proxmox_substrate_action".to_string()),
        }
    }
}
