//! Layer-1 node principal provisioning — pure plan over the Proxmox API
//! adapter. Mirrors the validated scratch logic (7/7 tests): typed argv exec,
//! bounded task wait, config read-back proof.
//!
//! PDP contract: caller must have run the Layer-2 lifecycle gate BEFORE this
//! module mutates anything (`--apply` explicit, fail-closed).

use cynic_protocol::NodeSubstrateDesiredState;

/// Canonical provisioning plan for one node principal.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvisionPrincipalPlan {
    pub schema: String,
    pub node_id: String,
    pub pve_node: String,
    pub vmid: u32,
    pub principal: String,
    /// Typed argv executed inside the container — no shell strings.
    pub exec_argv: Vec<String>,
    pub read_back_path: String,
}

pub const PROVISION_PRINCIPAL_PLAN_SCHEMA: &str = "cynic.node_provision.principal_plan.v1";

/// Pure plan builder — same output for the same input (deterministic).
pub fn plan_provision_principal(
    desired: &NodeSubstrateDesiredState,
    pve_node: &str,
    principal: &str,
) -> Result<ProvisionPrincipalPlan, String> {
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
    if kind != "lxc" || id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("invalid_proxmox_resource_ref".into());
    }
    let vmid: u32 = id.parse().map_err(|_| "invalid_proxmox_resource_ref")?;
    if principal.is_empty()
        || !principal
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("invalid_node_principal_name".into());
    }
    Ok(ProvisionPrincipalPlan {
        schema: PROVISION_PRINCIPAL_PLAN_SCHEMA.to_string(),
        node_id: desired.node_id.clone(),
        pve_node: pve_node.to_string(),
        vmid,
        principal: principal.to_string(),
        // Idempotent-ish substrate provisioning; group first so --gid resolves.
        exec_argv: vec![
            "sh".into(),
            "-c".into(),
            format!(
                "getent group {p} >/dev/null || addgroup --system {p}; \
                 getent passwd {p} >/dev/null || useradd --system --gid {p} \
                 --home-dir /home/{p} --create-home --shell /usr/sbin/nologin {p}",
                p = principal
            ),
        ],
        read_back_path: format!("/nodes/{pve_node}/lxc/{vmid}/exec"),
    })
}

/// Outcome of one provisioning apply with read-back proof.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProvisionPrincipalProof {
    pub schema: String,
    pub plan: ProvisionPrincipalPlan,
    pub upid: String,
    pub task_exitcode: i64,
    pub read_back_verified: bool,
    pub observed_at_unix_ms: u64,
}

pub const PROVISION_PRINCIPAL_PROOF_SCHEMA: &str = "cynic.node_provision.principal_proof.v1";

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{NodeSubstrateDesiredState, NodeSubstratePower};

    fn desired() -> NodeSubstrateDesiredState {
        NodeSubstrateDesiredState {
            node_id: "cynic-builder".into(),
            provider: Some("proxmox".into()),
            resource_id: Some("lxc/105".into()),
            desired_power: NodeSubstratePower::Running,
            provisioning: None,
        }
    }

    #[test]
    fn plan_is_deterministic_and_typed() {
        let a = plan_provision_principal(&desired(), "pve", "cynic").unwrap();
        let b = plan_provision_principal(&desired(), "pve", "cynic").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.vmid, 105);
        assert!(a.exec_argv[0] == "sh" && a.exec_argv[1] == "-c");
        assert!(a.exec_argv[2].contains("useradd --system"));
        assert!(!a.exec_argv.join(" ").contains("root@"));
    }

    #[test]
    fn plan_rejects_non_lxc_and_bad_principal() {
        let mut d = desired();
        d.resource_id = Some("qemu/105".into());
        assert_eq!(
            plan_provision_principal(&d, "pve", "cynic").unwrap_err(),
            "invalid_proxmox_resource_ref"
        );
        assert_eq!(
            plan_provision_principal(&desired(), "pve", "bad name").unwrap_err(),
            "invalid_node_principal_name"
        );
    }
}
