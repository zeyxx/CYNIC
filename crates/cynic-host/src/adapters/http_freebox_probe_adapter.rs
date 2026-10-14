//! Concrete HTTP Adapter for probing Freebox OS routers.
//! Belongs strictly to the Host Substrate layer (outbound adapter fulfilling FreeboxProbePort).
//! Never imported by the pure core domain.

use std::time::Duration;

use cynic_network::FreeboxProbePort;
use cynic_network::{FREEBOX_LAN_OBSERVATION_SCHEMA, FreeboxHostEntry, FreeboxLanObservation};
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
            app_id: "so.cynic.node".to_string(),
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

impl FreeboxProbePort for HttpFreeboxProbeAdapter {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
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
                                "app_id": self.app_id,
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
