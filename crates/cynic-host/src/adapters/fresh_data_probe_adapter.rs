//! Fresh-data probe adapter — executes `hermes chat --provider <provider> -q "PONG"`
//! and parses the result into a structured probe outcome.
//!
//! Belongs to the Host Substrate layer (outbound adapter). Never imported by
//! the pure core domain.

use std::process::Command;
use std::time::{Duration, Instant};

/// Outcome of a single provider probe.
#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    pub provider: String,
    pub model: Option<String>,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Adapter for probing a single Hermes provider.
pub struct HermesProbeAdapter {
    pub timeout: Duration,
}

impl Default for HermesProbeAdapter {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}

impl HermesProbeAdapter {
    /// Probe a single provider by running `hermes chat --provider <provider> -q "PONG"`.
    pub fn probe(&self, provider: &str, model: Option<&str>) -> ProbeOutcome {
        // Default model per provider (task-based routing)
        let default_model = match provider {
            "antigravity" => Some("gemini-3.8-flash-high"),
            "nous" => Some("poolside/laguna-s-2.1:free"),
            "opencode-go" => Some("beam-free"),
            _ => None,
        };

        let model = model.or(default_model);

        let mut cmd = Command::new("hermes");
        cmd.arg("chat")
            .arg("--provider")
            .arg(provider)
            .arg("-q")
            .arg("PONG");

        if let Some(m) = model {
            cmd.arg("-m").arg(m);
        }

        let t0 = Instant::now();
        let result = cmd.output();
        let dt = t0.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let success = output.status.success() && stdout.to_uppercase().contains("PONG");

                ProbeOutcome {
                    provider: provider.to_string(),
                    model: model.map(|s| s.to_string()),
                    latency_ms: dt,
                    success,
                    error: if success {
                        None
                    } else if !stderr.is_empty() {
                        Some(stderr)
                    } else {
                        Some("Probe failed: no PONG in response".to_string())
                    },
                }
            }
            Err(e) => ProbeOutcome {
                provider: provider.to_string(),
                model: model.map(|s| s.to_string()),
                latency_ms: dt,
                success: false,
                error: Some(format!("spawn error: {e}")),
            },
        }
    }
}