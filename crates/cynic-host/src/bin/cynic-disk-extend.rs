//! cynic-disk-extend — extend a VM disk via Proxmox REST API.
//!
//! Runs on cortex, connects to Proxmox REST API at https://pve.synapse.cynic:8006.
//! Usage: cynic-disk-extend --vmid 105 --size +40G [--endpoint <url>] [--token <id=secret>]

use std::process::Command;

fn read_proxmox_token() -> Result<(String, String), String> {
    let token_path = "/etc/cynic/secrets/proxmox-token";
    let token = std::fs::read_to_string(token_path)
        .map_err(|e| format!("read token failed: {e}"))?;
    let token = token.trim();
    // Format: "cynic@pve!runtime=secret"
    let parts: Vec<&str> = token.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("invalid token format in {token_path}"));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn rest_api(endpoint: &str, token_id: &str, token_secret: &str, method: &str, path: &str, body: Option<&str>) -> Result<String, String> {
    let url = format!("{}{}", endpoint, path);
    let mut args = vec![
        "-s".to_string(),
        "-k".to_string(),
        "-X".to_string(),
        method.to_string(),
        "-H".to_string(),
        format!("Authorization: PVEAPIToken={}={}", token_id, token_secret),
    ];
    if let Some(body) = body {
        args.push("-H".to_string());
        args.push("Content-Type: application/json".to_string());
        args.push("-d".to_string());
        args.push(body.to_string());
    }
    args.push(url);
    let output = Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "HTTP error: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn resolve_node(endpoint: &str, token_id: &str, token_secret: &str, vmid: u32) -> Result<String, String> {
    let output = rest_api(endpoint, token_id, token_secret, "GET", "/api2/json/cluster/resources", None)?;
    let json: serde_json::Value = serde_json::from_str(&output)
        .map_err(|e| format!("invalid_json: {e}"))?;
    let arr = json.as_array().ok_or("resources_not_array")?;
    for item in arr {
        if item.get("vmid").and_then(serde_json::Value::as_u64) == Some(vmid as u64) {
            return item
                .get("node")
                .and_then(serde_json::Value::as_str)
                .map(String::from)
                .ok_or_else(|| "node_field_missing".to_string());
        }
    }
    Err(format!("vmid {vmid} not found on cluster"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    let mut vmid: Option<u32> = None;
    let mut size: Option<String> = None;
    let mut endpoint = "https://pve.synapse.cynic:8006".to_string();
    let mut token: Option<(String, String)> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--vmid" => {
                i += 1;
                vmid = args.get(i).and_then(|s| s.parse().ok());
            }
            "--size" => {
                i += 1;
                size = args.get(i).cloned();
            }
            "--endpoint" => {
                i += 1;
                if let Some(e) = args.get(i) {
                    endpoint = e.clone();
                }
            }
            "--token" => {
                i += 1;
                if let Some(t) = args.get(i) {
                    let parts: Vec<&str> = t.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        token = Some((parts[0].to_string(), parts[1].to_string()));
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    let vmid = vmid.expect("--vmid required");
    let size = size.expect("--size required (e.g. +40G)");

    let (token_id, token_secret) = token.unwrap_or_else(|| read_proxmox_token().expect("no token provided"));

    println!("Resizing VM {vmid} disk by {size}...");

    let node = match resolve_node(&endpoint, &token_id, &token_secret, vmid) {
        Ok(n) => {
            println!("  Node: {n}");
            n
        }
        Err(e) => {
            eprintln!("Failed to resolve node: {e}");
            std::process::exit(1);
        }
    };

    // Detect VM type
    let lxc_probe = rest_api(
        &endpoint,
        &token_id,
        &token_secret,
        "GET",
        &format!("/api2/json/nodes/{node}/lxc/{vmid}/status/current"),
        None,
    );
    let vm_type = if _is_ok(&lxc_probe) { "lxc" } else { "qemu" };
    println!("  Type: {vm_type}");

    let disk_id = if vm_type == "lxc" { "rootfs" } else { "virtio0" };
    println!("  Disk: {disk_id}");

    let path = format!("/api2/json/nodes/{node}/{vm_type}/{vmid}/resize");
    let body = serde_json::json!({ "disk": disk_id, "size": size });

    match rest_api(&endpoint, &token_id, &token_secret, "POST", &path, Some(&body.to_string())) {
        Ok(_) => {
            println!("  Success: VM {vmid} disk extended by {size}");
        }
        Err(e) => {
            eprintln!("Failed to resize: {e}");
            std::process::exit(1);
        }
    }
}

fn _is_ok<T, E>(result: &Result<T, E>) -> bool {
    result.is_ok()
}
