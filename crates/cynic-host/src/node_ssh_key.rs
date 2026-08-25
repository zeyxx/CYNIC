//! Layer-1 SSH public key delivery to a container principal, over the
//! Proxmox API exec channel. Scratch-validated 7/7
//! (/tmp/proxmox-api-scratch provision_ssh_key).
//!
//! Injection safety: the pubkey line is VALIDATED against a strict allowlist
//! before being embedded into our internally-built argv. Callers never pass
//! raw shell — they pass a validated key line + principal name.

use cynic_protocol::NodeSubstrateDesiredState;

pub const DELIVER_SSH_KEY_PLAN_SCHEMA: &str = "cynic.node_provision.ssh_key_plan.v1";

/// Strict pubkey validation: ed25519 only, single line, shell-safe charset.
/// Rejects quotes/dollars/backticks/semicolons/newlines -> cannot break out
/// of the single-quoted embedding in our typed argv.
pub fn validate_pubkey_line(line: &str) -> Result<(), String> {
    let line = line.trim();
    if !line.starts_with("ssh-ed25519 ") {
        return Err("pubkey_kind_not_ed25519".into());
    }
    if line.len() > 512 || line.contains('\n') || line.contains('\r') {
        return Err("pubkey_length_or_newline_invalid".into());
    }
    if !line
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '+' | '/' | '=' | '@' | '.' | '-'))
    {
        return Err("pubkey_charset_outside_allowlist".into());
    }
    // exactly: kind + space + base64 [+ space + comment]
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err("pubkey_field_count_invalid".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeliverSshKeyPlan {
    pub schema: String,
    pub node_id: String,
    pub pve_node: String,
    pub vmid: u32,
    pub principal: String,
    pub mutate_argv: Vec<String>,
    pub read_back_argv: Vec<String>,
}

impl DeliverSshKeyPlan {
    pub fn mutate_refs(&self) -> Vec<&str> {
        self.mutate_argv.iter().map(String::as_str).collect()
    }
    pub fn read_back_refs(&self) -> Vec<&str> {
        self.read_back_argv.iter().map(String::as_str).collect()
    }
}

fn resource_vmid(desired: &NodeSubstrateDesiredState) -> Result<u32, String> {
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
    id.parse()
        .map_err(|_| "invalid_proxmox_resource_ref".into())
}

/// Pure plan builder — deterministic for identical inputs.
pub fn plan_deliver_principal_ssh_key(
    desired: &NodeSubstrateDesiredState,
    pve_node: &str,
    principal: &str,
    pubkey_line: &str,
) -> Result<DeliverSshKeyPlan, String> {
    validate_pubkey_line(pubkey_line)?;
    let vmid = resource_vmid(desired)?;
    if principal.is_empty()
        || matches!(principal, "root" | "daemon" | "bin" | "sys" | "nobody")
        || !principal
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("invalid_node_principal_name".into());
    }
    let key = pubkey_line.trim();
    let home = format!("/home/{principal}");
    let auth_keys = format!("{home}/.ssh/authorized_keys");
    let mutate_argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "install -d -m 700 '{h}/.ssh' && printf '%s\\n' '{k}' > '{ak}' \
             && chmod 600 '{ak}'",
            h = home,
            k = key,
            ak = auth_keys
        ),
    ];
    let read_back_argv = vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("[ \"$(cat {ak})\" = '{k}' ]", ak = auth_keys, k = key),
    ];
    Ok(DeliverSshKeyPlan {
        schema: DELIVER_SSH_KEY_PLAN_SCHEMA.to_string(),
        node_id: desired.node_id.clone(),
        pve_node: pve_node.to_string(),
        vmid,
        principal: principal.to_string(),
        mutate_argv,
        read_back_argv,
    })
}
