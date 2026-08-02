#[tool_router(router = tool_router_local_legacy_proxmox, vis = "pub(crate)")]
impl CynicMcpProxy {
    #[cfg(feature = "legacy_infra_tools")]
    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_list",
            description = "List all VMs and LXC containers on the Proxmox hypervisor. Returns their status, memory, and ID."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_list(&self) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_list")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        // 1. Get nodes
        let nodes_resp = client
            .get(format!("{proxmox_url}/api2/json/nodes"))
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| {
                McpError::internal_error(format!("Proxmox API failed (nodes): {e}"), None)
            })?;

        let nodes_json: serde_json::Value = nodes_resp.json().await.map_err(|e| {
            McpError::internal_error(format!("Invalid JSON from Proxmox: {e}"), None)
        })?;

        let mut output = String::new();

        if let Some(nodes) = nodes_json["data"].as_array() {
            for node in nodes {
                if let Some(node_name) = node["node"].as_str() {
                    output.push_str(&format!("=== Node: {node_name} ===\n"));

                    // 2. Get VMs
                    let qemu_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/qemu"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = qemu_resp
                        && let Ok(qemu_json) = resp.json::<serde_json::Value>().await
                    {
                        output.push_str("VMs:\n");
                        if let Some(vms) = qemu_json["data"].as_array() {
                            for vm in vms {
                                let vmid = vm["vmid"].as_u64().unwrap_or(0);
                                let name = vm["name"].as_str().unwrap_or("unknown");
                                let status = vm["status"].as_str().unwrap_or("unknown");
                                let mem = vm["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                output.push_str(&format!(
                                    "- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"
                                ));
                            }
                        }
                    }

                    // 3. Get LXCs
                    let lxc_resp = client
                        .get(format!("{proxmox_url}/api2/json/nodes/{node_name}/lxc"))
                        .header("Authorization", &auth_header)
                        .send()
                        .await;
                    if let Ok(resp) = lxc_resp
                        && let Ok(lxc_json) = resp.json::<serde_json::Value>().await
                    {
                        output.push_str("Containers (LXC):\n");
                        if let Some(lxcs) = lxc_json["data"].as_array() {
                            for lxc in lxcs {
                                let vmid = lxc["vmid"].as_u64().unwrap_or(0);
                                let name = lxc["name"].as_str().unwrap_or("unknown");
                                let status = lxc["status"].as_str().unwrap_or("unknown");
                                let mem = lxc["maxmem"].as_u64().unwrap_or(0) / 1024 / 1024;
                                output.push_str(&format!(
                                    "- ID: {vmid}, Name: {name}, Status: {status}, Mem: {mem}MB\n"
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[cfg(feature = "legacy_infra_tools")]
    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_action",
            description = "Perform actions (start, stop, reboot, suspend, resume) on Proxmox VMs and LXC containers."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_action(
        &self,
        params: Parameters<ProxmoxActionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_action")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;
        let vmid = p.vmid;
        let node = p.node;
        let type_ = p.target_type; // "qemu" or "lxc"
        let action = p.action; // "start", "stop", "reboot", "suspend", "resume"

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let url = format!("{proxmox_url}/api2/json/nodes/{node}/{type_}/{vmid}/status/{action}");

        let resp = client
            .post(&url)
            .header("Authorization", &auth_header)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[cfg(feature = "legacy_infra_tools")]
    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_provision",
            description = "Provision a new Proxmox VM from an archive backup (vma.gz). Provides autonomous infrastructure cloning."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_provision(
        &self,
        params: Parameters<ProxmoxProvisionParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_provision")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(300)) // Provisioning takes time
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let url = format!("{proxmox_url}/api2/json/nodes/{}/qemu", p.node);

        // pvesh create /nodes/pve/qemu -vmid 130 -archive local:backup/... -name cynic-vm-staging -storage local-lvm
        let form = [
            ("vmid", p.vmid.to_string()),
            ("archive", p.archive),
            ("name", p.name),
            ("storage", p.storage),
            ("force", "1".to_string()), // Force overwriting existing if any
        ];

        let resp = client
            .post(&url)
            .header("Authorization", &auth_header)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[cfg(feature = "legacy_infra_tools")]
    #[cfg_attr(
        feature = "legacy_infra_tools",
        tool(
            name = "cynic_proxmox_exec",
            description = "Execute a command inside a Proxmox VM using the QEMU Guest Agent without requiring SSH."
        )
    )]
    #[cfg_attr(not(feature = "legacy_infra_tools"), allow(dead_code))]
    async fn cynic_proxmox_exec(
        &self,
        params: Parameters<ProxmoxExecParams>,
    ) -> Result<CallToolResult, McpError> {
        self.require_auth()?;
        self.rate_limit.check_other()?;
        require_legacy_infra_tool("cynic_proxmox_exec")?;

        let proxmox_url = std::env::var("CYNIC_PROXMOX_URL").unwrap_or_default();
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").unwrap_or_default();
        let credential_value = Self::proxmox_credential();

        if proxmox_url.is_empty() || client_id.is_empty() || credential_value.is_empty() {
            return Err(McpError::internal_error(
                "Proxmox credentials not configured in CYNIC_PROXMOX_* env vars",
                None,
            ));
        }

        let p = params.0;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| McpError::internal_error(format!("Proxmox client error: {e}"), None))?;

        let auth_scheme = "PVEAPIToken";
        let auth_header = format!("{auth_scheme}={client_id}={credential_value}");

        let exec_url = format!(
            "{proxmox_url}/api2/json/nodes/{}/qemu/{}/agent/exec",
            p.node, p.vmid
        );

        if p.argv.is_empty() || p.argv.iter().any(|argument| argument.contains('\0')) {
            return Err(McpError::invalid_params(
                "argv must contain at least one NUL-free argument",
                None,
            ));
        }
        let form = p
            .argv
            .into_iter()
            .map(|argument| ("command", argument))
            .collect::<Vec<_>>();

        let resp = client
            .post(&exec_url)
            .header("Authorization", &auth_header)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("Proxmox API POST failed: {e}"), None))?;

        let text = resp.text().await.unwrap_or_default();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
        let pid = json["data"]["pid"].as_i64().ok_or_else(|| {
            McpError::internal_error(
                format!("Failed to get PID from exec response: {text}"),
                None,
            )
        })?;

        let status_url = format!(
            "{proxmox_url}/api2/json/nodes/{}/qemu/{}/agent/exec-status?pid={}",
            p.node, p.vmid, pid
        );

        // Wait for execution to finish (up to ~30 seconds)
        for _ in 0..15 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let status_resp = client
                .get(&status_url)
                .header("Authorization", &auth_header)
                .send()
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("Proxmox API GET failed: {e}"), None)
                })?;

            let status_text = status_resp.text().await.unwrap_or_default();
            let status_json: serde_json::Value =
                serde_json::from_str(&status_text).unwrap_or(serde_json::json!({}));

            if status_json["data"]["exited"].as_i64() == Some(1) {
                let exitcode = status_json["data"]["exitcode"].as_i64().unwrap_or(-1);
                let out_data = status_json["data"]["out-data"].as_str().unwrap_or("");
                let err_data = status_json["data"]["err-data"].as_str().unwrap_or("");

                let result_text =
                    format!("Exit code: {exitcode}\nStdout:\n{out_data}\nStderr:\n{err_data}");
                return Ok(CallToolResult::success(vec![Content::text(result_text)]));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Command timed out after 30 seconds. PID was {pid}."
        ))]))
    }
}
