//! Freebox OS local router probe port and adapter.
//!
//! Queries the local Freebox Server API for WAN/LAN discovery without storing secrets in git.

use cynic_protocol::{FREEBOX_LAN_OBSERVATION_SCHEMA, FreeboxLanObservation};
use serde_json::Value;
use std::time::Duration;

pub trait FreeboxProbePort: Send + Sync {
    fn probe_freebox_lan(&self, endpoint: &str) -> Result<FreeboxLanObservation, String>;
}

/// Concrete HTTP adapter for probing Freebox OS routers.
pub struct HttpFreeboxProbeAdapter {
    pub timeout: Duration,
}

impl Default for HttpFreeboxProbeAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(3),
        }
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

        Ok(FreeboxLanObservation {
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
            hosts: vec![],
            observed_at_unix_ms: now_unix_ms,
            unknowns: vec!["lan_browser_requires_authenticated_session_token".to_string()],
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
                    hosts: vec![],
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
    }
}
