impl CynicMcpProxy {
    async fn provider_cluster_resources(&self) -> serde_json::Value {
        match cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::from_env(
            std::time::Duration::from_secs(10),
        ) {
            Ok(adapter) => adapter
                .cluster_resources()
                .await
                .unwrap_or_else(|reason| serde_json::json!({"status": "error", "reason": reason})),
            Err(reason) => serde_json::json!({"status": "unavailable", "reason": reason}),
        }
    }

    async fn agent_connection_status(&self) -> serde_json::Value {
        use cynic_protocol::agent_connection::{
            AgentConnectionObservation, assess_agent_connection,
        };

        let agent_id = &self.session_identity.agent_id;
        let local_credential_present = self.kernel.is_active();
        let url = match query_url(
            self.kernel.base_url(),
            "/coord/who",
            &[("agent_id", agent_id.clone())],
        ) {
            Ok(url) => url,
            Err(_) => {
                return serde_json::json!({
                    "schema": "cynic.agent_connection_report.v1",
                    "status": "configuration_invalid",
                    "agent_id": agent_id,
                    "kernel_reachable": null,
                    "kernel_authentication": "unknown",
                    "registered": null,
                    "local_credential_present": local_credential_present,
                    "blockers": ["kernel_url_invalid"],
                    "secret_exposed": false,
                });
            }
        };
        let response = self.kernel.client().get(url).send().await;
        let Ok(response) = response else {
            return serde_json::to_value(assess_agent_connection(AgentConnectionObservation {
                agent_id: agent_id.clone(),
                kernel_reachable: false,
                http_status: None,
                response_body: String::new(),
                local_credential_present,
                registered: None,
            }))
            .unwrap_or_default();
        };
        let http_status = response.status();
        let body = response.text().await.unwrap_or_default();
        if http_status.is_success() {
            let payload = serde_json::from_str::<serde_json::Value>(&body).unwrap_or_default();
            let registered = payload
                .as_array()
                .map(|agents| !agents.is_empty())
                .or_else(|| {
                    payload
                        .get("agents")
                        .and_then(|agents| agents.as_array())
                        .map(|agents| !agents.is_empty())
                });
            return serde_json::to_value(assess_agent_connection(AgentConnectionObservation {
                agent_id: agent_id.clone(),
                kernel_reachable: true,
                http_status: Some(http_status.as_u16()),
                response_body: body,
                local_credential_present,
                registered,
            }))
            .unwrap_or_default();
        }
        serde_json::to_value(assess_agent_connection(AgentConnectionObservation {
            agent_id: agent_id.clone(),
            kernel_reachable: true,
            http_status: Some(http_status.as_u16()),
            response_body: body,
            local_credential_present,
            registered: None,
        }))
        .unwrap_or_default()
    }

    async fn provider_qemu_guest_exec(
        &self,
        node: &str,
        vmid: &str,
        argv: &[String],
        timeout_seconds: u64,
    ) -> Result<serde_json::Value, McpError> {
        let adapter = cynic_node::adapters::proxmox_control::ProxmoxControlAdapter::from_env(
            std::time::Duration::from_secs(timeout_seconds.max(5)),
        )
        .map_err(|error| McpError::internal_error(error, None))?;
        adapter
            .qemu_guest_exec(
                node,
                vmid,
                argv,
                std::time::Duration::from_secs(timeout_seconds.max(1)),
            )
            .await
            .map_err(|error| McpError::internal_error(error, None))
    }

    fn lxc_exec_bridge_config() -> Option<(String, String, String)> {
        let registry = cynic_host::runtime_registry::load_runtime_registry(None);
        let hypervisor_ssh = registry
            .as_ref()
            .and_then(|value| value.get("nodes"))
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|node| node.get("role").and_then(serde_json::Value::as_str) == Some("hypervisor"))
            .and_then(|node| node.get("transports"))
            .and_then(|value| value.get("ssh"));
        let ssh_target = std::env::var("CYNIC_LXC_EXEC_BRIDGE_SSH_TARGET")
            .ok()
            .or_else(|| {
                hypervisor_ssh
                    .and_then(|value| value.get("target"))
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())?;
        let helper = std::env::var("CYNIC_LXC_EXEC_BRIDGE_HELPER")
            .unwrap_or_else(|_| "/usr/local/libexec/cynic-lxc-exec-bridge".to_string());
        let known_hosts = std::env::var("CYNIC_LXC_EXEC_BRIDGE_KNOWN_HOSTS")
            .ok()
            .or_else(|| {
                hypervisor_ssh
                    .and_then(|value| value.get("known_hosts_ref"))
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
            .map(|value| value.trim().to_string())
            .filter(|value| value.starts_with('/') && !value.contains(".."))?;
        Some((ssh_target, helper, known_hosts))
    }

    async fn run_lxc_exec_bridge(
        runner: &dyn ArgvExecutionPort,
        ssh_target: &str,
        helper: &str,
        known_hosts: &str,
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let output = runner
            .execute(ArgvExecutionRequest {
                argv: vec![
                    "ssh".to_string(),
                    "-o".to_string(),
                    "BatchMode=yes".to_string(),
                    "-o".to_string(),
                    "ConnectTimeout=5".to_string(),
                    "-o".to_string(),
                    "StrictHostKeyChecking=yes".to_string(),
                    "-o".to_string(),
                    format!("UserKnownHostsFile={known_hosts}"),
                    ssh_target.to_string(),
                    "sudo".to_string(),
                    "-n".to_string(),
                    helper.to_string(),
                ],
                current_dir: None,
                stdin: Some(serde_json::to_vec(request).unwrap_or_default()),
                timeout: std::time::Duration::from_secs(30),
            })
            .await;
        let success = output.success();
        let exit_code = output.exit_code;
        let stderr = output
            .spawn_error
            .as_deref()
            .unwrap_or(output.stderr.as_str());
        let parsed =
            serde_json::from_str::<serde_json::Value>(&output.stdout).unwrap_or_else(|_| {
                serde_json::json!({
                    "schema": "cynic.lxc_exec_bridge.response.v1",
                    "status": "error",
                    "error": "bridge_output_not_json",
                    "stdout": output.stdout.as_str(),
                    "stderr": stderr,
                    "ssh_exit_code": exit_code
                })
            });
        Ok(serde_json::json!({
            "status": if success { "completed" } else { "bridge_failed" },
            "provider": "lxc_exec_bridge_ssh",
            "ssh_target": ssh_target,
            "helper": helper,
            "ssh_exit_code": exit_code,
            "bridge": parsed,
            "stderr": stderr,
        }))
    }

    async fn observe_lxc_exec_bridge_transport(
        runner: &dyn ArgvExecutionPort,
        local_projection: serde_json::Value,
    ) -> serde_json::Value {
        let Some((ssh_target, helper, known_hosts)) = Self::lxc_exec_bridge_config() else {
            return serde_json::json!({
                "schema": "cynic.lxc_exec_bridge.transport_status.v1",
                "status": "enrollment_required",
                "transport_configured": false,
                "remote_helper": null,
                "local_projection": local_projection,
                "blockers": ["pinned_ssh_transport_missing"],
                "secret_exposed": false,
            });
        };
        let output = runner
            .execute(ArgvExecutionRequest {
                argv: vec![
                    "ssh".to_string(),
                    "-o".to_string(),
                    "BatchMode=yes".to_string(),
                    "-o".to_string(),
                    "ConnectTimeout=5".to_string(),
                    "-o".to_string(),
                    "StrictHostKeyChecking=yes".to_string(),
                    "-o".to_string(),
                    format!("UserKnownHostsFile={known_hosts}"),
                    ssh_target,
                    "sudo".to_string(),
                    "-n".to_string(),
                    helper,
                    "--status".to_string(),
                ],
                current_dir: None,
                stdin: None,
                timeout: std::time::Duration::from_secs(10),
            })
            .await;
        let remote_helper = serde_json::from_str::<serde_json::Value>(&output.stdout).ok();
        let active = output.success()
            && remote_helper
                .as_ref()
                .and_then(|value| value.get("status"))
                .and_then(|value| value.as_str())
                == Some("active")
            && remote_helper
                .as_ref()
                .and_then(|value| value.get("runtime"))
                .and_then(|value| value.as_str())
                == Some("rust_native");
        serde_json::json!({
            "schema": "cynic.lxc_exec_bridge.transport_status.v1",
            "status": if active { "active" } else { "blocked" },
            "transport_configured": true,
            "remote_helper": remote_helper,
            "remote_exit_code": output.exit_code,
            "remote_timed_out": output.timed_out,
            "local_projection": local_projection,
            "blockers": if active { Vec::<String>::new() } else { vec!["remote_rust_helper_read_back_failed".to_string()] },
            "secret_exposed": false,
        })
    }

    async fn run_host_exec_transport(
        runner: &dyn ArgvExecutionPort,
        argv: &[String],
        request: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        if argv.is_empty() {
            return Err(McpError::internal_error(
                "host exec transport command is empty",
                None,
            ));
        }
        let output = runner
            .execute(ArgvExecutionRequest {
                argv: argv.to_vec(),
                current_dir: None,
                stdin: Some(serde_json::to_vec(request).unwrap_or_default()),
                timeout: std::time::Duration::from_secs(30),
            })
            .await;
        let success = output.success();
        let exit_code = output.exit_code;
        let stderr = output
            .spawn_error
            .as_deref()
            .unwrap_or(output.stderr.as_str());
        let parsed =
            serde_json::from_str::<serde_json::Value>(&output.stdout).unwrap_or_else(|_| {
                serde_json::json!({
                    "schema": "cynic.host_exec_transport.response.v1",
                    "status": "error",
                    "error": "transport_output_not_json",
                    "stdout": output.stdout.as_str(),
                    "stderr": stderr,
                    "exit_code": exit_code
                })
            });
        Ok(serde_json::json!({
            "status": if success { "completed" } else { "transport_failed" },
            "provider": "host_exec_transport",
            "command": argv,
            "exit_code": exit_code,
            "transport": parsed,
            "stderr": stderr,
        }))
    }
}
