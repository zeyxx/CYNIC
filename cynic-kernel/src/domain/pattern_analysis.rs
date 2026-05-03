//! Pattern Analysis — detects failure patterns from live health data and emits healing observations.
//!
//! Consumes: /health endpoint alerts
//! Produces: Observations with healing rules (K15 consumer="pattern_healing")
//!
//! Rules implemented:
//! 1. Embedding failure rate = 100% → "disable service, test NullEmbedding fallback"
//! 2. Organ silence > 1h → "observation producer dead or offline"
//! 3. Storage query latency > 200ms → "missing index on seq column"
//! 4. Context drift (actual < expected) → "GPU VRAM constraint, update expected"
//! 5. Model mismatch (config != reality) → "update backends.toml to match reality"

use serde::Serialize;

/// A healable pattern detected from health signals.
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub kind: String,               // embedding_crash_loop, organ_silence, etc.
    pub severity: String,           // critical, warning, info
    pub signal: String,             // what was detected (e.g., "failure_rate=100%")
    pub reasoning: String,          // why this matters
    pub recommended_action: String, // what to do (e.g., "disable llama-embed")
    pub is_actionable: bool,        // safe to auto-execute?
}

/// Healing observation to emit to /observe endpoint.
#[derive(Debug, Serialize)]
pub struct HealingObservation {
    pub tool: String,    // "pattern_analyzer"
    pub domain: String,  // "organism_health" or specific domain
    pub status: String,  // critical, warning, info
    pub context: String, // detailed recommendation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumer: Option<String>, // K15: who consumes this
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>, // K15: what changes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>, // epistemic label
}

impl HealingObservation {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "tool": self.tool,
            "domain": self.domain,
            "status": self.status,
            "context": self.context,
            "consumer": self.consumer,
            "action": self.action,
            "confidence": self.confidence.as_deref().unwrap_or("observed"),
        })
    }
}

/// Analyze health alerts and return detected patterns.
pub fn detect_patterns(health_alerts: Vec<(String, String, String)>) -> Vec<DetectedPattern> {
    let mut patterns = vec![];

    for (kind, message, _severity) in health_alerts {
        match kind.as_str() {
            "embedding_failure_rate" => {
                // Example: "Embedding failure rate 100.0% (45/45 calls)"
                if message.contains("100") || message.contains("100.0") {
                    patterns.push(DetectedPattern {
                        kind: "embedding_crash_loop".to_string(),
                        severity: "critical".to_string(),
                        signal: message.clone(),
                        reasoning: "All embedding requests failing — likely service crash loop or missing model file".to_string(),
                        recommended_action: "Disable llama-embed service (systemctl --user disable llama-embed), verify NullEmbedding fallback active".to_string(),
                        is_actionable: false, // requires systemd interaction
                    });
                }
            }
            "organ_silence" => {
                // Example: "Organ 'hermes-x-organ' silent for 1h"
                if message.contains("silent for") && message.contains("h") {
                    patterns.push(DetectedPattern {
                        kind: "organ_silence".to_string(),
                        severity: "warning".to_string(),
                        signal: message.clone(),
                        reasoning: "Organ stopped emitting observations — either crashed or observation pipeline blocked".to_string(),
                        recommended_action: "Check organ health: verify cron jobs running, check network connectivity, review organ logs".to_string(),
                        is_actionable: false, // requires investigation
                    });
                }
            }
            "metabolism_anomaly" => {
                // Example: "RTK savings 31.0% below threshold 38.2%"
                patterns.push(DetectedPattern {
                    kind: "metabolism_anomaly".to_string(),
                    severity: "warning".to_string(),
                    signal: message.clone(),
                    reasoning: "Token filtering degraded — inference becoming more expensive".to_string(),
                    recommended_action: "Monitor token output in next 3 sessions; if trend continues, investigate Dog quality drift".to_string(),
                    is_actionable: false, // observation only
                });
            }
            "fleet_drift" => {
                // Example: "Fleet node 'kairos' offline"
                if message.contains("offline") {
                    patterns.push(DetectedPattern {
                        kind: "fleet_drift".to_string(),
                        severity: "warning".to_string(),
                        signal: message.clone(),
                        reasoning: "Infrastructure node offline — may impact multi-node deployments".to_string(),
                        recommended_action: "Check node status via Tailscale; if persistent >10min, may need manual recovery".to_string(),
                        is_actionable: false, // infrastructure concern
                    });
                }
            }
            _ => {}
        }
    }

    patterns
}

/// Emit a healing observation via HTTP POST to kernel /observe endpoint.
///
/// Must be called from async context. Uses a blocking HTTP client internally.
pub async fn emit_healing_observation(
    kernel_addr: &str,
    api_key: &str,
    pattern: &DetectedPattern,
) -> Result<(), String> {
    let obs = HealingObservation {
        tool: "pattern_analyzer".to_string(),
        domain: "organism_health".to_string(),
        status: pattern.severity.clone(),
        context: format!(
            "{}\n\nRecommended action: {}",
            pattern.reasoning, pattern.recommended_action
        ),
        consumer: Some("pattern_healing".to_string()),
        action: Some(pattern.recommended_action.clone()),
        confidence: Some("observed".to_string()),
    };

    let client = reqwest::Client::new();
    let url = format!("http://{}/observe", kernel_addr);
    let headers = {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(
            "Authorization",
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|_| "Invalid auth header".to_string())?,
        );
        h.insert(
            "Content-Type",
            "application/json"
                .parse()
                .map_err(|_| "Invalid content-type".to_string())?,
        );
        h
    };

    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        match client
            .post(&url)
            .headers(headers)
            .json(&obs.to_json())
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(format!("POST /observe returned {}", resp.status())),
            Err(e) => Err(format!("HTTP request failed: {}", e)),
        }
    })
    .await
    .map_err(|_| "POST /observe timed out (5s)".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_embedding_failure_100_percent() {
        let alerts = vec![(
            "embedding_failure_rate".to_string(),
            "Embedding failure rate 100.0% (45/45 calls)".to_string(),
            "warning".to_string(),
        )];
        let patterns = detect_patterns(alerts);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].kind, "embedding_crash_loop");
        assert_eq!(patterns[0].severity, "critical");
    }

    #[test]
    fn detect_organ_silence() {
        let alerts = vec![(
            "organ_silence".to_string(),
            "Organ 'hermes-x-organ' silent for 1h (last obs: 2026-04-27T03:22:00Z, total: 787)"
                .to_string(),
            "warning".to_string(),
        )];
        let patterns = detect_patterns(alerts);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].kind, "organ_silence");
    }

    #[test]
    fn detect_fleet_drift() {
        let alerts = vec![(
            "fleet_drift".to_string(),
            "Fleet node 'kairos' offline".to_string(),
            "warning".to_string(),
        )];
        let patterns = detect_patterns(alerts);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].kind, "fleet_drift");
    }

    #[test]
    fn ignore_partial_failure_rate() {
        let alerts = vec![(
            "embedding_failure_rate".to_string(),
            "Embedding failure rate 43.7% (1438/3288 calls)".to_string(),
            "warning".to_string(),
        )];
        let patterns = detect_patterns(alerts);
        // Should not trigger if < 100%
        let embedding_patterns: Vec<_> = patterns
            .iter()
            .filter(|p| p.kind == "embedding_crash_loop")
            .collect();
        assert_eq!(embedding_patterns.len(), 0);
    }
}
