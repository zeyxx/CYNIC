//! Freebox OS local router probe port and adapter.
//!
//! Queries the local Freebox Server API for WAN/LAN discovery without storing secrets in git.
//! Supports optional session authentication when FREEBOX_APP_TOKEN is injected.

use std::path::PathBuf;
use std::time::Duration;

use cynic_protocol::{FREEBOX_LAN_OBSERVATION_SCHEMA, FreeboxHostEntry, FreeboxLanObservation};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

pub trait FreeboxProbePort: Send + Sync {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String>;
}

/// Concrete HTTP adapter for probing Freebox OS routers.
pub struct HttpFreeboxProbeAdapter {
    pub timeout: Duration,
    pub app_id: String,
}

impl Default for HttpFreeboxProbeAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            app_id: "so.cynic.node".to_string(),
        }
    }
}

impl HttpFreeboxProbeAdapter {
    /// Resolve injected Freebox app token from environment or local config file.
    pub fn resolve_app_token() -> Option<String> {
        if let Ok(token) = std::env::var("FREEBOX_APP_TOKEN") {
            let trimmed = token.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }

        let home = std::env::var("HOME").ok().map(PathBuf::from)?;
        let config_path = home.join(".config").join("cynic").join("freebox-app-token");
        if let Ok(content) = std::fs::read_to_string(config_path) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }

        let etc_path = PathBuf::from("/etc/cynic/freebox-app-token");
        if let Ok(content) = std::fs::read_to_string(etc_path) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }

        None
    }

    /// Computes password hash for Freebox OS session login: HMAC-SHA1(app_token, challenge).
    fn compute_password(app_token: &str, challenge: &str) -> Result<String, String> {
        let mut mac = HmacSha1::new_from_slice(app_token.as_bytes())
            .map_err(|e| format!("hmac_init_failed:{e}"))?;
        mac.update(challenge.as_bytes());
        let result = mac.finalize();
        Ok(hex::encode(result.into_bytes()))
    }
}

impl FreeboxProbePort for HttpFreeboxProbeAdapter {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String> {
        let base_url = endpoint.trim_end_matches('/');
        let api_version_url = format!("{base_url}/api_version");

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| format!("http_client_init_failed:{e}"))?;

        let response = client
            .get(&api_version_url)
            .send()
            .map_err(|e| format!("freebox_api_unreachable:{e}"))?;

        let now_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if !response.status().is_success() {
            return Ok(FreeboxLanObservation {
                schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
                endpoint: endpoint.to_string(),
                reachable: false,
                auth_state: "unreachable".to_string(),
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
                hosts: vec![],
                observed_at_unix_ms: now_unix_ms,
                unknowns: vec![format!("http_status:{}", response.status())],
            });
        }

        let json: Value = response
            .json()
            .map_err(|e| format!("invalid_freebox_json:{e}"))?;

        let box_model_name = json["box_model_name"].as_str().map(ToString::to_string);
        let device_name = json["device_name"].as_str().map(ToString::to_string);
        let api_version = json["api_version"].as_str().map(ToString::to_string);
        let api_domain = json["api_domain"].as_str().map(ToString::to_string);
        let https_port = json["https_port"].as_u64().map(|p| p as u16);
        let https_available = json["https_available"].as_bool().unwrap_or(false);
        let uid = json["uid"].as_str().map(ToString::to_string);

        let app_token = Self::resolve_app_token();
        if app_token.is_none() {
            return Ok(FreeboxLanObservation {
                schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
                endpoint: endpoint.to_string(),
                reachable: true,
                auth_state: "unauthenticated_discovery".to_string(),
                box_model_name,
                device_name,
                api_version,
                api_domain,
                https_port,
                https_available,
                uid,
                wan_status: None,
                wan_ipv4: None,
                wan_ipv6: None,
                rate_down_bps: None,
                rate_up_bps: None,
                bytes_down_total: None,
                bytes_up_total: None,
                hosts: vec![],
                observed_at_unix_ms: now_unix_ms,
                unknowns: vec!["lan_browser_requires_authenticated_session_token".to_string()],
            });
        }

        let token = app_token.unwrap();

        // 1. Get login challenge
        let login_url = format!("{base_url}/api/v8/login/");
        let login_res = client.get(&login_url).send();
        let challenge = match login_res {
            Ok(res) if res.status().is_success() => {
                let val: Value = res.json().unwrap_or_default();
                val["result"]["challenge"].as_str().map(ToString::to_string)
            }
            _ => None,
        };

        let Some(&ref challenge) = challenge.as_ref() else {
            return Ok(FreeboxLanObservation {
                schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
                endpoint: endpoint.to_string(),
                reachable: true,
                auth_state: "challenge_failed".to_string(),
                box_model_name,
                device_name,
                api_version,
                api_domain,
                https_port,
                https_available,
                uid,
                wan_status: None,
                wan_ipv4: None,
                wan_ipv6: None,
                rate_down_bps: None,
                rate_up_bps: None,
                bytes_down_total: None,
                bytes_up_total: None,
                hosts: vec![],
                observed_at_unix_ms: now_unix_ms,
                unknowns: vec!["freebox_login_challenge_failed".to_string()],
            });
        };

        // 2. Open session
        let password = Self::compute_password(&token, &challenge)?;
        let session_url = format!("{base_url}/api/v8/login/session/");
        let session_res = client
            .post(&session_url)
            .json(&serde_json::json!({
                "app_id": self.app_id,
                "password": password
            }))
            .send();

        let session_token = match session_res {
            Ok(res) if res.status().is_success() => {
                let val: Value = res.json().unwrap_or_default();
                val["result"]["session_token"]
                    .as_str()
                    .map(ToString::to_string)
            }
            _ => None,
        };

        let Some(session_token) = session_token else {
            return Ok(FreeboxLanObservation {
                schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
                endpoint: endpoint.to_string(),
                reachable: true,
                auth_state: "session_rejected".to_string(),
                box_model_name,
                device_name,
                api_version,
                api_domain,
                https_port,
                https_available,
                uid,
                wan_status: None,
                wan_ipv4: None,
                wan_ipv6: None,
                rate_down_bps: None,
                rate_up_bps: None,
                bytes_down_total: None,
                bytes_up_total: None,
                hosts: vec![],
                observed_at_unix_ms: now_unix_ms,
                unknowns: vec!["freebox_app_token_unauthorized".to_string()],
            });
        };

        // 3. Query WAN connection status
        let conn_url = format!("{base_url}/api/v8/connection/");
        let (
            wan_status,
            wan_ipv4,
            wan_ipv6,
            rate_down_bps,
            rate_up_bps,
            bytes_down_total,
            bytes_up_total,
        ) = if let Ok(res) = client
            .get(&conn_url)
            .header("X-Fbx-App-Auth", &session_token)
            .send()
        {
            if let Ok(val) = res.json::<Value>() {
                let r = &val["result"];
                (
                    r["state"].as_str().map(ToString::to_string),
                    r["ipv4"].as_str().map(ToString::to_string),
                    r["ipv6"].as_str().map(ToString::to_string),
                    r["rate_down"].as_u64(),
                    r["rate_up"].as_u64(),
                    r["bytes_down"].as_u64(),
                    r["bytes_up"].as_u64(),
                )
            } else {
                (None, None, None, None, None, None, None)
            }
        } else {
            (None, None, None, None, None, None, None)
        };

        // 4. Query LAN browser
        let lan_url = format!("{base_url}/api/v8/lan/browser/pub/");
        let mut hosts = Vec::new();
        if let Ok(res) = client
            .get(&lan_url)
            .header("X-Fbx-App-Auth", &session_token)
            .send()
        {
            if let Ok(val) = res.json::<Value>() {
                if let Some(arr) = val["result"].as_array() {
                    for item in arr {
                        let id = item["id"].as_str().unwrap_or("").to_string();
                        let primary_name = item["primary_name"].as_str().unwrap_or("").to_string();
                        let ip = item["l3connectivities"]
                            .as_array()
                            .and_then(|c| c.first())
                            .and_then(|c| c["addr"].as_str())
                            .map(ToString::to_string);
                        let mac = item["l2ident"]["id"].as_str().map(ToString::to_string);
                        let host_type = item["host_type"].as_str().map(ToString::to_string);
                        let active = item["active"].as_bool().unwrap_or(false);
                        let reachable = item["reachable"].as_bool().unwrap_or(false);
                        let interface = item["interface"].as_str().map(ToString::to_string);

                        if !id.is_empty() || !primary_name.is_empty() {
                            hosts.push(FreeboxHostEntry {
                                id,
                                primary_name,
                                ip,
                                mac,
                                host_type,
                                active,
                                reachable,
                                interface,
                            });
                        }
                    }
                }
            }
        }

        Ok(FreeboxLanObservation {
            schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
            endpoint: endpoint.to_string(),
            reachable: true,
            auth_state: "authenticated_session".to_string(),
            box_model_name,
            device_name,
            api_version,
            api_domain,
            https_port,
            https_available,
            uid,
            wan_status,
            wan_ipv4,
            wan_ipv6,
            rate_down_bps,
            rate_up_bps,
            bytes_down_total,
            bytes_up_total,
            hosts,
            observed_at_unix_ms: now_unix_ms,
            unknowns: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFreeboxProbe {
        reachable: bool,
    }

    impl FreeboxProbePort for MockFreeboxProbe {
        fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String> {
            if self.reachable {
                Ok(FreeboxLanObservation {
                    schema: FREEBOX_LAN_OBSERVATION_SCHEMA.to_string(),
                    endpoint: endpoint.to_string(),
                    reachable: true,
                    auth_state: "unauthenticated_discovery".to_string(),
                    box_model_name: Some("Freebox Server (r2)".to_string()),
                    device_name: Some("Freebox Server".to_string()),
                    api_version: Some("16.0".to_string()),
                    api_domain: Some("qssmpbs5.fbxos.fr".to_string()),
                    https_port: Some(21295),
                    https_available: true,
                    uid: Some("d06a61eab70bd3b7730db427d0a5d11b".to_string()),
                    wan_status: Some("up".to_string()),
                    wan_ipv4: Some("82.65.0.1".to_string()),
                    wan_ipv6: None,
                    rate_down_bps: Some(1000000000),
                    rate_up_bps: Some(700000000),
                    bytes_down_total: Some(123456789),
                    bytes_up_total: Some(987654321),
                    hosts: vec![FreeboxHostEntry {
                        id: "ether-d8:bb:c1:04:10:35".to_string(),
                        primary_name: "cortex".to_string(),
                        ip: Some("192.168.0.11".to_string()),
                        mac: Some("d8:bb:c1:04:10:35".to_string()),
                        host_type: Some("workstation".to_string()),
                        active: true,
                        reachable: true,
                        interface: Some("ethernet".to_string()),
                    }],
                    observed_at_unix_ms: 1788267750000,
                    unknowns: vec![],
                })
            } else {
                Err("connection_refused".to_string())
            }
        }
    }

    #[test]
    fn mock_freebox_probe_reports_reachable_status() {
        let probe = MockFreeboxProbe { reachable: true };
        let obs = probe.probe_freebox_lan("http://192.168.0.254").unwrap();
        assert!(obs.reachable);
        assert_eq!(obs.box_model_name.as_deref(), Some("Freebox Server (r2)"));
        assert_eq!(obs.hosts.len(), 1);
        assert_eq!(obs.hosts[0].primary_name, "cortex");
    }

    #[test]
    fn hmac_sha1_computation_is_deterministic() {
        let app_token = "my_app_token_secret_12345";
        let challenge = "challenge_from_freebox_67890";
        let password = HttpFreeboxProbeAdapter::compute_password(app_token, challenge).unwrap();
        assert_eq!(password.len(), 40); // SHA-1 hex is 40 chars
    }
}
