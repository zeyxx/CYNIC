//! Concrete HTTP Adapter for probing Freebox OS routers.
//! Belongs strictly to the Host Substrate layer (outbound adapter fulfilling FreeboxProbePort).
//! Never imported by the pure core domain.

use std::time::Duration;

use crate::freebox::{
    FreeboxDmzConfig, FreeboxNatRule, FreeboxNatRuleSpec, FreeboxPort, FreeboxProbePort,
    FreeboxWanStatus, FREEBOX_DMZ_CONFIG_SCHEMA, FREEBOX_NAT_RULE_SCHEMA,
    FREEBOX_WAN_STATUS_SCHEMA,
};
use crate::types::{FreeboxHostEntry, FreeboxLanObservation, FREEBOX_LAN_OBSERVATION_SCHEMA};
use hmac::{Hmac, KeyInit, Mac};
use serde_json::Value;
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

/// Concrete HTTP adapter for probing Freebox OS routers.
pub struct HttpFreeboxProbeAdapter {
    pub timeout: Duration,
    pub app_id: String,
}

impl Default for HttpFreeboxProbeAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            app_id: "cynic".to_string(),
        }
    }
}

impl HttpFreeboxProbeAdapter {
    /// Secret id shared by every leg of the resolution chain.
    const APP_TOKEN_SECRET_ID: &str = "freebox-app-token";

    /// Resolve the Freebox app token through the dedicated secret system.
    ///
    /// Chain (first hit wins, value never logged):
    /// 1. `FREEBOX_APP_TOKEN` process environment (dev / container injection).
    /// 2. Encrypted principal-owned file store (`SecretProvider::PrincipalOwnedFile`).
    /// 3. OS credential store (`SecretProvider::OsCredentialStore`, e.g. gnome-keyring).
    ///
    /// The legacy raw files (`~/.config/cynic/freebox-app-token`, the system
    /// config directory's `freebox-app-token`) are intentionally NOT read here:
    /// files bypass the secret ports (no encryption, no audit, no rotation).
    /// Migrate their content once with the `SecretStorePort`, then shred them.
    pub fn resolve_app_token() -> Option<String> {
        use cynic_secrets::{CompositeSecretProvider, SecretProviderPort, SecretReference};
        use cynic_secrets::{SecretClass, SecretProviderKind};
        let composite = CompositeSecretProvider::default();
        if let Some(token) = composite.resolve(&SecretReference::env(
            Self::APP_TOKEN_SECRET_ID,
            "FREEBOX_APP_TOKEN",
        )) {
            return Some(token);
        }
        for provider in [
            SecretProviderKind::PrincipalOwnedFile,
            SecretProviderKind::OsCredentialStore,
        ] {
            let reference = SecretReference::with_class(
                Self::APP_TOKEN_SECRET_ID,
                Self::APP_TOKEN_SECRET_ID,
                SecretClass::MachineEnrollment,
                provider,
            );
            if let Some(token) = composite.resolve(&reference) {
                return Some(token);
            }
        }
        None
    }

    /// Computes password hash for Freebox OS session login: HMAC-SHA1(app_token, challenge).
    pub fn compute_password(app_token: &str, challenge: &str) -> Result<String, String> {
        let mut mac = HmacSha1::new_from_slice(app_token.as_bytes())
            .map_err(|e| format!("hmac_init_failed:{e}"))?;
        mac.update(challenge.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

impl FreeboxPort for HttpFreeboxProbeAdapter {
    fn probe_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::probe_freebox_lan_blocking(&inner_endpoint, inner_timeout, &inner_app_id)
        })
        .join()
        .map_err(|_| "freebox_probe_thread_panicked".to_string())?
    }

    fn read_wan(&self, endpoint: &str) -> Result<FreeboxWanStatus, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::read_wan_blocking(&inner_endpoint, inner_timeout, &inner_app_id)
        })
        .join()
        .map_err(|_| "freebox_wan_thread_panicked".to_string())?
    }

    fn read_dmz(&self, endpoint: &str) -> Result<FreeboxDmzConfig, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::read_dmz_config_blocking(&inner_endpoint, inner_timeout, &inner_app_id)
        })
        .join()
        .map_err(|_| "freebox_dmz_thread_panicked".to_string())?
    }

    fn write_dmz(
        &self,
        endpoint: &str,
        enabled: bool,
        ip: &str,
    ) -> Result<FreeboxDmzConfig, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        let inner_ip = ip.to_string();
        std::thread::spawn(move || {
            Self::write_dmz_config_blocking(
                &inner_endpoint,
                inner_timeout,
                &inner_app_id,
                enabled,
                &inner_ip,
            )
        })
        .join()
        .map_err(|_| "freebox_dmz_write_thread_panicked".to_string())?
    }

    fn list_nat_rules(&self, endpoint: &str) -> Result<Vec<FreeboxNatRule>, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::list_nat_rules_blocking(&inner_endpoint, inner_timeout, &inner_app_id)
        })
        .join()
        .map_err(|_| "freebox_list_nat_thread_panicked".to_string())?
    }

    fn add_nat_rule(
        &self,
        endpoint: &str,
        spec: &FreeboxNatRuleSpec,
    ) -> Result<FreeboxNatRule, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        let inner_spec = spec.clone();
        std::thread::spawn(move || {
            Self::add_nat_rule_blocking(&inner_endpoint, inner_timeout, &inner_app_id, &inner_spec)
        })
        .join()
        .map_err(|_| "freebox_add_nat_thread_panicked".to_string())?
    }

    fn delete_nat_rule(&self, endpoint: &str, rule_id: u32) -> Result<(), String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::delete_fw_redir_blocking(&inner_endpoint, inner_timeout, &inner_app_id, rule_id)
        })
        .join()
        .map_err(|_| "freebox_delete_nat_thread_panicked".to_string())?
    }
}

impl FreeboxProbePort for HttpFreeboxProbeAdapter {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String> {
        FreeboxPort::probe_lan(self, endpoint)
    }
}

impl HttpFreeboxProbeAdapter {
    /// Delete a NAT/port-forwarding rule by id over an authenticated
    /// session. Runs confined to a plain OS thread (same reqwest::blocking
    /// constraint as the probe). Refuses without a stored app token.
    pub fn delete_fw_redir(&self, endpoint: &str, rule_id: u32) -> Result<(), String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::delete_fw_redir_blocking(&inner_endpoint, inner_timeout, &inner_app_id, rule_id)
        })
        .join()
        .map_err(|_| "freebox_delete_thread_panicked".to_string())?
    }

    /// List NAT/port-forwarding rules (id, ports, comment) over an
    /// authenticated session. Read-only; the approval read-back path.
    pub fn list_fw_redirs(&self, endpoint: &str) -> Result<Vec<(u32, u16, String)>, String> {
        let inner_endpoint = endpoint.to_string();
        let inner_timeout = self.timeout;
        let inner_app_id = self.app_id.clone();
        std::thread::spawn(move || {
            Self::list_fw_redirs_blocking(&inner_endpoint, inner_timeout, &inner_app_id)
        })
        .join()
        .map_err(|_| "freebox_list_thread_panicked".to_string())?
    }

    /// Read the DMZ configuration: whether it is enabled and the host IP it
    /// targets. Deprecated legacy tuple form; prefer `FreeboxPort::read_dmz`.
    pub fn read_dmz(&self, endpoint: &str) -> Result<(bool, String), String> {
        FreeboxPort::read_dmz(self, endpoint).map(|c| (c.enabled, c.ip))
    }

    fn open_authenticated_session(
        client: &reqwest::blocking::Client,
        endpoint: &str,
        app_id: &str,
    ) -> Result<String, String> {
        let token = Self::resolve_app_token().ok_or_else(|| "freebox_app_token_missing".to_string())?;
        let challenge: String = client
            .get(&format!("{endpoint}/api/v10/login/"))
            .send()
            .map_err(|e| format!("freebox_challenge_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/challenge")
            .and_then(|c| c.as_str())
            .ok_or_else(|| "freebox_challenge_missing".to_string())?
            .to_string();
        let password = Self::compute_password(&token, &challenge)?;
        let session_token: String = client
            .post(&format!("{endpoint}/api/v10/login/session/"))
            .json(&serde_json::json!({ "app_id": app_id, "password": password }))
            .send()
            .map_err(|e| format!("freebox_session_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/session_token")
            .and_then(|st| st.as_str())
            .ok_or_else(|| "freebox_session_rejected".to_string())?
            .to_string();
        Ok(session_token)
    }

    fn read_dmz_config_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<FreeboxDmzConfig, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let session_token = Self::open_authenticated_session(&client, endpoint, app_id)?;
        let result: Value = client
            .get(&format!("{endpoint}/api/v10/fw/dmz/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_dmz_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .cloned()
            .ok_or_else(|| "freebox_dmz_shape_unexpected".to_string())?;
        let enabled = result
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| "freebox_dmz_shape_unexpected".to_string())?;
        let ip = result
            .get("ip")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(FreeboxDmzConfig {
            schema: FREEBOX_DMZ_CONFIG_SCHEMA.to_string(),
            enabled,
            ip,
        })
    }

    fn write_dmz_config_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
        enabled: bool,
        ip: &str,
    ) -> Result<FreeboxDmzConfig, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let session_token = Self::open_authenticated_session(&client, endpoint, app_id)?;
        let body = serde_json::json!({
            "enabled": enabled,
            "ip": ip,
        });
        let result: Value = client
            .put(&format!("{endpoint}/api/v10/fw/dmz/"))
            .header("X-Fbx-App-Auth", session_token)
            .json(&body)
            .send()
            .map_err(|e| format!("freebox_dmz_write_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .cloned()
            .ok_or_else(|| "freebox_dmz_write_shape_unexpected".to_string())?;

        let res_enabled = result
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(enabled);
        let res_ip = result
            .get("ip")
            .and_then(|v| v.as_str())
            .unwrap_or(ip)
            .to_string();

        Ok(FreeboxDmzConfig {
            schema: FREEBOX_DMZ_CONFIG_SCHEMA.to_string(),
            enabled: res_enabled,
            ip: res_ip,
        })
    }

    fn read_wan_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<FreeboxWanStatus, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let session_token = Self::open_authenticated_session(&client, endpoint, app_id)?;
        let result: Value = client
            .get(&format!("{endpoint}/api/v10/connection/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_wan_status_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .cloned()
            .ok_or_else(|| "freebox_wan_shape_unexpected".to_string())?;

        let state = result
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let ipv4 = result
            .get("ipv4")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let ipv6 = result
            .get("ipv6")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let rate_down_bps = result.get("rate_down").and_then(|v| v.as_u64());
        let rate_up_bps = result.get("rate_up").and_then(|v| v.as_u64());
        let bytes_down_total = result.get("bytes_down").and_then(|v| v.as_u64());
        let bytes_up_total = result.get("bytes_up").and_then(|v| v.as_u64());

        Ok(FreeboxWanStatus {
            schema: FREEBOX_WAN_STATUS_SCHEMA.to_string(),
            state,
            ipv4,
            ipv6,
            rate_down_bps,
            rate_up_bps,
            bytes_down_total,
            bytes_up_total,
        })
    }

    fn list_nat_rules_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<Vec<FreeboxNatRule>, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let session_token = Self::open_authenticated_session(&client, endpoint, app_id)?;
        let rules: Vec<Value> = client
            .get(&format!("{endpoint}/api/v10/fw/redir/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_redir_list_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| "freebox_redir_shape_unexpected".to_string())?
            .clone();

        Ok(rules
            .iter()
            .map(|r| FreeboxNatRule {
                schema: FREEBOX_NAT_RULE_SCHEMA.to_string(),
                id: r.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                enabled: r.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
                ip: r.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                protocol: r.get("ip_proto").and_then(|v| v.as_str()).unwrap_or("tcp").to_string(),
                wan_port_start: r.get("wan_port_start").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                wan_port_end: r.get("wan_port_end").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                lan_port: r.get("lan_port").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                comment: r.get("comment").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            })
            .collect())
    }

    fn add_nat_rule_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
        spec: &FreeboxNatRuleSpec,
    ) -> Result<FreeboxNatRule, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let session_token = Self::open_authenticated_session(&client, endpoint, app_id)?;
        let body = serde_json::json!({
            "enabled": spec.enabled,
            "ip": spec.ip,
            "ip_proto": spec.protocol,
            "wan_port_start": spec.wan_port_start,
            "wan_port_end": spec.wan_port_end,
            "lan_port": spec.lan_port,
            "comment": spec.comment,
        });

        let resp: Value = client
            .post(&format!("{endpoint}/api/v10/fw/redir/"))
            .header("X-Fbx-App-Auth", session_token)
            .json(&body)
            .send()
            .map_err(|e| format!("freebox_redir_add_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?;

        if !resp.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let msg = resp
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_error");
            return Err(format!("freebox_redir_add_rejected:{msg}"));
        }

        let result = resp
            .pointer("/result")
            .ok_or_else(|| "freebox_redir_add_missing_result".to_string())?;

        Ok(FreeboxNatRule {
            schema: FREEBOX_NAT_RULE_SCHEMA.to_string(),
            id: result.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            enabled: result.get("enabled").and_then(|v| v.as_bool()).unwrap_or(spec.enabled),
            ip: result.get("ip").and_then(|v| v.as_str()).unwrap_or(&spec.ip).to_string(),
            protocol: result.get("ip_proto").and_then(|v| v.as_str()).unwrap_or(&spec.protocol).to_string(),
            wan_port_start: result.get("wan_port_start").and_then(|v| v.as_u64()).unwrap_or(spec.wan_port_start as u64) as u16,
            wan_port_end: result.get("wan_port_end").and_then(|v| v.as_u64()).unwrap_or(spec.wan_port_end as u64) as u16,
            lan_port: result.get("lan_port").and_then(|v| v.as_u64()).unwrap_or(spec.lan_port as u64) as u16,
            comment: result.get("comment").and_then(|v| v.as_str()).unwrap_or(&spec.comment).to_string(),
        })
    }

    fn read_dmz_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<(bool, String), String> {
        let token = Self::resolve_app_token().ok_or_else(|| "freebox_app_token_missing".to_string())?;
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let challenge: String = client
            .get(&format!("{endpoint}/api/v10/login/"))
            .send()
            .map_err(|e| format!("freebox_challenge_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/challenge")
            .and_then(|c| c.as_str())
            .ok_or_else(|| "freebox_challenge_missing".to_string())?
            .to_string();
        let password = Self::compute_password(&token, &challenge)?;
        let session_token: String = client
            .post(&format!("{endpoint}/api/v10/login/session/"))
            .json(&serde_json::json!({ "app_id": app_id, "password": password }))
            .send()
            .map_err(|e| format!("freebox_session_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/session_token")
            .and_then(|st| st.as_str())
            .ok_or_else(|| "freebox_session_rejected".to_string())?
            .to_string();
        let result: Value = client
            .get(&format!("{endpoint}/api/v10/fw/dmz/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_dmz_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .cloned()
            .ok_or_else(|| "freebox_dmz_shape_unexpected".to_string())?;
        let enabled = result
            .get("enabled")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| "freebox_dmz_shape_unexpected".to_string())?;
        let ip = result
            .get("ip")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok((enabled, ip))
    }

    fn list_fw_redirs_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<Vec<(u32, u16, String)>, String> {
        let token = Self::resolve_app_token().ok_or_else(|| "freebox_app_token_missing".to_string())?;
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let challenge: String = client
            .get(&format!("{endpoint}/api/v10/login/"))
            .send()
            .map_err(|e| format!("freebox_challenge_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/challenge")
            .and_then(|c| c.as_str())
            .ok_or_else(|| "freebox_challenge_missing".to_string())?
            .to_string();
        let password = Self::compute_password(&token, &challenge)?;
        let session_token: String = client
            .post(&format!("{endpoint}/api/v10/login/session/"))
            .json(&serde_json::json!({ "app_id": app_id, "password": password }))
            .send()
            .map_err(|e| format!("freebox_session_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/session_token")
            .and_then(|st| st.as_str())
            .ok_or_else(|| "freebox_session_rejected".to_string())?
            .to_string();
        let rules: Vec<Value> = client
            .get(&format!("{endpoint}/api/v10/fw/redir/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_redir_list_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result")
            .and_then(|r| r.as_array())
            .ok_or_else(|| "freebox_redir_shape_unexpected".to_string())?
            .clone();
        Ok(rules
            .iter()
            .map(|r| {
                (
                    r.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    r.get("wan_port_start").and_then(|v| v.as_u64()).unwrap_or(0) as u16,
                    r.get("comment").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                )
            })
            .collect())
    }

    fn delete_fw_redir_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
        rule_id: u32,
    ) -> Result<(), String> {
        let token = Self::resolve_app_token().ok_or_else(|| "freebox_app_token_missing".to_string())?;
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;
        let challenge: String = client
            .get(&format!("{endpoint}/api/v10/login/"))
            .send()
            .map_err(|e| format!("freebox_challenge_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/challenge")
            .and_then(|c| c.as_str())
            .ok_or_else(|| "freebox_challenge_missing".to_string())?
            .to_string();
        let password = Self::compute_password(&token, &challenge)?;
        let session_token: String = client
            .post(&format!("{endpoint}/api/v10/login/session/"))
            .json(&serde_json::json!({ "app_id": app_id, "password": password }))
            .send()
            .map_err(|e| format!("freebox_session_request_failed:{e}"))?
            .json::<Value>()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?
            .pointer("/result/session_token")
            .and_then(|st| st.as_str())
            .ok_or_else(|| "freebox_session_rejected".to_string())?
            .to_string();
        let delete_resp = client
            .delete(&format!("{endpoint}/api/v10/fw/redir/{rule_id}/"))
            .header("X-Fbx-App-Auth", session_token)
            .send()
            .map_err(|e| format!("freebox_redir_delete_failed:{e}"))?;
        if !delete_resp.status().is_success() {
            return Err(format!("freebox_redir_delete_http_{}", delete_resp.status()));
        }
        Ok(())
    }

    fn probe_freebox_lan_blocking(
        endpoint: &str,
        timeout: Duration,
        app_id: &str,
    ) -> Result<FreeboxLanObservation, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;

        let app_token = Self::resolve_app_token();

        let mut session_token = None;
        if let Some(token) = &app_token {
            let auth_url = format!("{endpoint}/api/v10/login/");
            let challenge_resp = client
                .get(&auth_url)
                .send()
                .map_err(|e| format!("freebox_challenge_request_failed:{e}"))?;

            if challenge_resp.status().is_success() {
                if let Ok(v) = challenge_resp.json::<Value>() {
                    if let Some(challenge) = v.pointer("/result/challenge").and_then(|c| c.as_str())
                    {
                        if let Ok(password) = Self::compute_password(token, challenge) {
                            let open_session_url = format!("{endpoint}/api/v10/login/session/");
                            let body = serde_json::json!({
                                "app_id": app_id,
                                "password": password,
                            });
                            let session_resp = client
                                .post(&open_session_url)
                                .json(&body)
                                .send()
                                .map_err(|e| format!("freebox_session_request_failed:{e}"))?;

                            if session_resp.status().is_success() {
                                if let Ok(sv) = session_resp.json::<Value>() {
                                    session_token = sv
                                        .pointer("/result/session_token")
                                        .and_then(|st| st.as_str())
                                        .map(str::to_string);
                                }
                            }
                        }
                    }
                }
            }
        }

        let lan_browser_url = format!("{endpoint}/api/v10/lan/browser/pub/");
        let mut req = client.get(&lan_browser_url);
        if let Some(st) = &session_token {
            req = req.header("X-Fbx-App-Auth", st);
        }

        let resp = req
            .send()
            .map_err(|e| format!("freebox_lan_request_failed:{e}"))?;

        if !resp.status().is_success() {
            return Err(format!("freebox_returned_http_{}", resp.status()));
        }

        let val: Value = resp
            .json()
            .map_err(|e| format!("freebox_json_parse_failed:{e}"))?;

        let mut hosts = Vec::new();
        if let Some(entries) = val.get("result").and_then(|r| r.as_array()) {
            for entry in entries {
                let id = entry
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let primary_name = entry
                    .get("primary_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let host_type = entry
                    .get("host_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let reachable = entry
                    .get("reachable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let active = entry
                    .get("active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let mut ipv4 = None;
                if let Some(l3_conns) = entry.get("l3connectivities").and_then(|c| c.as_array()) {
                    for conn in l3_conns {
                        if conn.get("af").and_then(|a| a.as_str()) == Some("ipv4")
                            && conn.get("active").and_then(|a| a.as_bool()) == Some(true)
                        {
                            ipv4 = conn
                                .get("addr")
                                .and_then(|a| a.as_str())
                                .map(str::to_string);
                            break;
                        }
                    }
                }

                hosts.push(FreeboxHostEntry {
                    id,
                    primary_name,
                    ip: ipv4,
                    mac: None,
                    host_type: Some(host_type),
                    reachable,
                    active,
                    interface: None,
                });
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(FreeboxLanObservation {
            schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
            endpoint: endpoint.to_string(),
            reachable: true,
            auth_state: if session_token.is_some() {
                "authenticated".to_string()
            } else {
                "unauthenticated".to_string()
            },
            box_model_name: None,
            device_name: None,
            api_version: None,
            api_domain: None,
            https_port: None,
            https_available: false,
            uid: None,
            wan_status: None,
            wan_ipv4: None,
            wan_ipv6: None,
            rate_down_bps: None,
            rate_up_bps: None,
            bytes_down_total: None,
            bytes_up_total: None,
            hosts,
            observed_at_unix_ms: now_ms,
            unknowns: vec![],
        })
    }
}
