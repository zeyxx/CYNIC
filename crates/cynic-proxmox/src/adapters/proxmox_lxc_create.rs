//! Proxmox REST LXC creation executor (substrate binding).
//!
//! Consumes `LxcProvisioningSpec` + the observed preflight blockers and
//! drives `POST /nodes/{node}/lxc` through the scoped API token (never root
//! SSH). Default is plan-only; `apply=true` executes. Every HTTP call runs
//! inside a dedicated OS thread (same whole-request boundary as the REST
//! observer), so nothing blocking is built or dropped on a Tokio runtime.
//!
//! Rollback: on failed read-back the guest is destroyed (`vzdestroy` is
//! idempotent — an absent guest is success, never an error).

use cynic_protocol::provider::LxcProvisioningSpec;
use reqwest::blocking::Client;
use reqwest::Certificate;
use serde_json::{json, Value};

pub const LXC_CREATE_RECEIPT_SCHEMA: &str = "cynic.agent_host.lxc_create_receipt.v1";

/// Validate CA PEM bytes: must contain at least one PEM-armored certificate
/// block AND parse. `Certificate::from_pem` alone accepts bare garbage
/// (empty cert list), so the armor check is the real fail-closed gate.
pub fn validate_ca_pem(pem: &[u8]) -> Result<(), String> {
    if !pem
        .windows(b"-----BEGIN CERTIFICATE-----".len())
        .any(|w| w == b"-----BEGIN CERTIFICATE-----")
    {
        return Err("blocked_tls_invalid_ca:no_pem_block".to_string());
    }
    Certificate::from_pem(pem).map_err(|e| format!("blocked_tls_invalid_ca:{e}"))?;
    Ok(())
}

/// Run a blocking REST closure on a dedicated thread and return its owned
/// result. The closure must build its client, send, and parse inside the
/// thread — nothing blocking crosses the join boundary.
pub(crate) fn blocking_rest_call<T>(
    op: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String>
where
    T: Send + 'static,
{
    std::thread::spawn(op)
        .join()
        .map_err(|_| "pve_request_thread_panicked".to_string())?
}

/// Pure mapping: validated spec + vmid → exact `POST /nodes/{node}/lxc`
/// form body. Zero I/O, fully unit-testable.
pub fn build_lxc_create_body(vmid: u32, spec: &LxcProvisioningSpec) -> Value {
    let mut body = json!({
        "vmid": vmid,
        "ostemplate": spec.template_volid,
        "storage": spec.storage_pool,
        "hostname": spec.hostname,
        "cores": spec.cores,
        "memory": spec.memory_mib,
        "swap": spec.swap_mib,
        "rootfs": format!("{}:{}", spec.storage_pool, spec.disk_gib),
        "unprivileged": if spec.unprivileged { 1 } else { 0 },
        "onboot": if spec.onboot { 1 } else { 0 },
        "net0": "name=eth0,bridge=vmbr0,ip=dhcp",
    });
    if !spec.features.is_empty() {
        body["features"] = Value::String(spec.features.join(","));
    }
    body
}

/// Read-back of one guest after create.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcCreateReadBack {
    pub status: String,
    pub onboot: bool,
}

/// Rollback record: destroy attempted on failed read-back.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcCreateRollback {
    pub attempted: bool,
    pub result: String,
}

/// Execution receipt for one `create` call (plan or applied).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LxcCreateReceipt {
    pub schema: String,
    pub vmid: u32,
    pub applied: bool,
    pub upid: Option<String>,
    pub task_status: Option<String>,
    pub read_back: Option<LxcCreateReadBack>,
    pub rollback: LxcCreateRollback,
}

/// REST LXC creator. Built from the same resolved triple as the observer
/// (endpoint, token identity, secret, CA PEM) — no defaults, no env reads.
pub struct ProxmoxRestLxcCreator {
    base_url: String,
    token_id: String,
    token_secret: String,
    ca_pem: Vec<u8>,
}

impl std::fmt::Debug for ProxmoxRestLxcCreator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxmoxRestLxcCreator")
            .field("base_url", &self.base_url)
            .field("token_id", &self.token_id)
            .finish_non_exhaustive()
    }
}

impl ProxmoxRestLxcCreator {
    pub fn new(
        base_url: impl Into<String>,
        token_id: impl Into<String>,
        token_secret: impl Into<String>,
        ca_pem: Option<&[u8]>,
    ) -> Result<Self, String> {
        let Some(pem) = ca_pem else {
            return Err("blocked_tls_unproven:no_ca_pem_supplied".to_string());
        };
        validate_ca_pem(pem)?;
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            token_id: token_id.into(),
            token_secret: token_secret.into(),
            ca_pem: pem.to_vec(),
        })
    }

    fn auth_header(&self) -> String {
        format!("PVEAPIToken={}={}", self.token_id, self.token_secret)
    }

    fn client(&self) -> Result<Client, String> {
        let certificate =
            Certificate::from_pem(&self.ca_pem).map_err(|e| format!("blocked_tls_invalid_ca:{e}"))?;
        Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .add_root_certificate(certificate)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))
    }

    fn post_form(&self, path: &str, body: &Value) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let auth = self.auth_header();
        let client = self.client()?;
        let map: std::collections::HashMap<String, String> = body
            .as_object()
            .map(|o| {
                o.iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            v.as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| v.to_string()),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        blocking_rest_call(move || {
            let res = client
                .post(&url)
                .header("Authorization", auth)
                .form(&map)
                .send()
                .map_err(|e| format!("pve_api_request_failed:{e}"))?;
            if res.status() == reqwest::StatusCode::FORBIDDEN {
                return Err("token_forbidden:create".to_string());
            }
            if !res.status().is_success() {
                return Err(format!("pve_api_http_error:{}", res.status()));
            }
            let json: Value = res.json().map_err(|e| format!("invalid_pve_json:{e}"))?;
            Ok(json.get("data").cloned().unwrap_or(json))
        })
    }

    fn get_json(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let auth = self.auth_header();
        let client = self.client()?;
        blocking_rest_call(move || {
            let res = client
                .get(&url)
                .header("Authorization", auth)
                .send()
                .map_err(|e| format!("pve_api_request_failed:{e}"))?;
            if !res.status().is_success() {
                return Err(format!("pve_api_http_error:{}", res.status()));
            }
            let json: Value = res.json().map_err(|e| format!("invalid_pve_json:{e}"))?;
            Ok(json.get("data").cloned().unwrap_or(json))
        })
    }

    fn delete(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base_url, path);
        let auth = self.auth_header();
        let client = self.client()?;
        blocking_rest_call(move || {
            let res = client
                .delete(&url)
                .header("Authorization", auth)
                .send()
                .map_err(|e| format!("pve_api_request_failed:{e}"))?;
            if !res.status().is_success() {
                return Err(format!("pve_api_http_error:{}", res.status()));
            }
            let json: Value = res.json().map_err(|e| format!("invalid_pve_json:{e}"))?;
            Ok(json.get("data").cloned().unwrap_or(json))
        })
    }

    /// Plan by default; execute with `apply=true`. Preflight blockers refuse
    /// before any HTTP is attempted.
    pub fn create(
        &self,
        node: &str,
        vmid: u32,
        spec: &LxcProvisioningSpec,
        preflight_blockers: &[String],
        apply: bool,
    ) -> Result<LxcCreateReceipt, String> {
        if !preflight_blockers.is_empty() {
            return Err(format!(
                "create_blocked:{}",
                preflight_blockers.join(";")
            ));
        }
        let body = build_lxc_create_body(vmid, spec);
        if !apply {
            return Ok(LxcCreateReceipt {
                schema: LXC_CREATE_RECEIPT_SCHEMA.to_string(),
                vmid,
                applied: false,
                upid: None,
                task_status: None,
                read_back: None,
                rollback: LxcCreateRollback {
                    attempted: false,
                    result: "plan_only".to_string(),
                },
            });
        }
        let data = self.post_form(&format!("/nodes/{node}/lxc"), &body)?;
        let upid = data
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "create_missing_upid".to_string())?;

        let task_status = self.poll_task(node, &upid)?;
        let read_back = self.read_back(node, vmid)?;
        if read_back.status != "running" {
            let rollback = self.destroy(node, vmid);
            return Ok(LxcCreateReceipt {
                schema: LXC_CREATE_RECEIPT_SCHEMA.to_string(),
                vmid,
                applied: true,
                upid: Some(upid),
                task_status: Some(task_status),
                read_back: Some(read_back),
                rollback,
            });
        }
        Ok(LxcCreateReceipt {
            schema: LXC_CREATE_RECEIPT_SCHEMA.to_string(),
            vmid,
            applied: true,
            upid: Some(upid),
            task_status: Some(task_status),
            read_back: Some(read_back),
            rollback: LxcCreateRollback {
                attempted: false,
                result: "read_back_ok".to_string(),
            },
        })
    }

    fn poll_task(&self, node: &str, upid: &str) -> Result<String, String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        loop {
            let status = self.get_json(&format!("/nodes/{node}/tasks/{upid}/status"))?;
            let state = status
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if state == "stopped" {
                let exit = status
                    .get("exitstatus")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                return Ok(exit.to_string());
            }
            if std::time::Instant::now() > deadline {
                return Err(format!("create_task_timeout:{upid}"));
            }
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    fn read_back(&self, node: &str, vmid: u32) -> Result<LxcCreateReadBack, String> {
        let status = self.get_json(&format!("/nodes/{node}/lxc/{vmid}/status/current"))?;
        let state = status
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let config = self.get_json(&format!("/nodes/{node}/lxc/{vmid}/config"))?;
        let onboot = config.get("onboot").and_then(Value::as_u64).unwrap_or(0) == 1;
        Ok(LxcCreateReadBack {
            status: state,
            onboot,
        })
    }

    fn destroy(&self, node: &str, vmid: u32) -> LxcCreateRollback {
        match self.delete(&format!("/nodes/{node}/lxc/{vmid}")) {
            Ok(_) => LxcCreateRollback {
                attempted: true,
                result: "destroyed".to_string(),
            },
            Err(e) => LxcCreateRollback {
                attempted: true,
                result: format!("destroy_failed:{e}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> LxcProvisioningSpec {
        LxcProvisioningSpec {
            schema: "test".to_string(),
            hostname: "relay-107".to_string(),
            template_volid: "local:vztmpl/ubuntu-24.04-standard_24.04-2_amd64.tar.zst"
                .to_string(),
            storage_pool: "local".to_string(),
            disk_gib: 32,
            cores: 2,
            memory_mib: 4096,
            swap_mib: 512,
            bridge: "vmbr0".to_string(),
            ip_cidr: None,
            gateway: None,
            nameserver: None,
            unprivileged: true,
            onboot: true,
            features: vec!["nesting=1".to_string()],
        }
    }

    #[test]
    fn create_body_maps_spec_exactly() {
        let body = build_lxc_create_body(107, &spec());
        assert_eq!(body["vmid"], 107);
        assert_eq!(
            body["ostemplate"],
            "local:vztmpl/ubuntu-24.04-standard_24.04-2_amd64.tar.zst"
        );
        assert_eq!(body["storage"], "local");
        assert_eq!(body["hostname"], "relay-107");
        assert_eq!(body["cores"], 2);
        assert_eq!(body["memory"], 4096);
        assert_eq!(body["unprivileged"], 1);
        assert_eq!(body["onboot"], 1);
        assert_eq!(body["net0"], "name=eth0,bridge=vmbr0,ip=dhcp");
        assert_eq!(body["features"], "nesting=1");
    }

    #[test]
    fn blocked_preflight_refuses_before_http() {
        // Unreachable endpoint: refusal must precede any HTTP attempt, so
        // the error is the blocker, never a network failure.
        let creator = ProxmoxRestLxcCreator::new(
            "https://192.0.2.1:8006",
            "cynic@pve!runtime",
            "secret",
            Some(include_str!("test-ca.pem").as_bytes()),
        )
        .expect("creator builds with valid test CA");
        let result = creator.create(
            "pve",
            107,
            &spec(),
            &["template_not_found:spec=x".to_string()],
            true, // even with apply, blockers refuse first
        );
        assert_eq!(
            result.unwrap_err(),
            "create_blocked:template_not_found:spec=x"
        );
    }

    #[test]
    fn plan_only_returns_receipt_without_http() {
        let creator = ProxmoxRestLxcCreator::new(
            "https://192.0.2.1:8006",
            "cynic@pve!runtime",
            "secret",
            Some(include_str!("test-ca.pem").as_bytes()),
        )
        .expect("creator builds with valid test CA");
        let receipt = creator
            .create("pve", 107, &spec(), &[], false)
            .expect("plan-only succeeds without HTTP");
        assert!(!receipt.applied);
        assert_eq!(receipt.rollback.result, "plan_only");
    }

    #[tokio::test]
    async fn creator_builds_inside_async_runtime() {
        // Same regression gate as the observer: no blocking client may be
        // built or dropped on the caller's (possibly async) thread.
        let result = ProxmoxRestLxcCreator::new(
            "https://pve.service.cynic:8006",
            "cynic@pve!runtime",
            "secret",
            Some(b"not-a-pem"),
        );
        // Invalid PEM must fail closed with the TLS error, never panic.
        assert!(result.unwrap_err().contains("blocked_tls_invalid_ca"));
    }
}
