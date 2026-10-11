//! Pattern Analysis — detects failure patterns from live health signals and emits healing plans.
//!
//! Powered by cynic_protocol::health_signal: typed, pure, and verifiable.

use cynic_protocol::{AutonomicRemediationPlan, HealthSignal, reconcile_health_signal};
use serde::Serialize;

/// A healable pattern detected from health signals.
#[derive(Debug, Clone)]
pub struct DetectedPattern {
    pub kind: String,               // embedding_crash_loop, service_silence, etc.
    pub severity: String,           // critical, warning, info
    pub signal: String,             // what was detected (e.g., "failure_rate=100%")
    pub reasoning: String,          // why this matters
    pub recommended_action: String, // what to do (e.g., "disable llama-embed")
    pub is_actionable: bool,        // safe to auto-execute?
}

/// Healing observation to emit to storage.
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

/// Pure typed reconciliation from health signals into autonomic remediation plans.
pub fn reconcile_signals(
    signals: &[HealthSignal],
    now_unix_s: u64,
) -> Vec<AutonomicRemediationPlan> {
    signals
        .iter()
        .filter_map(|sig| reconcile_health_signal(sig, now_unix_s))
        .collect()
}

/// Analyze health alerts and return detected patterns (backwards-compatible wrapper).
pub fn detect_patterns(health_alerts: Vec<(String, String, String)>) -> Vec<DetectedPattern> {
    let mut patterns = vec![];

    for (kind, message, _severity) in health_alerts {
        match kind.as_str() {
            "embedding_failure_rate" if message.contains("100") || message.contains("100.0") => {
                patterns.push(DetectedPattern {
                    kind: "embedding_crash_loop".to_string(),
                    severity: "critical".to_string(),
                    signal: message.clone(),
                    reasoning: "All embedding requests failing — likely service crash loop or missing model file".to_string(),
                    recommended_action: "Disable llama-embed service (systemctl --user disable llama-embed), verify NullEmbedding fallback active".to_string(),
                    is_actionable: true,
                });
            }
            "service_silence" if message.contains("silent for") && message.contains("h") => {
                patterns.push(DetectedPattern {
                    kind: "service_silence".to_string(),
                    severity: "warning".to_string(),
                    signal: message.clone(),
                    reasoning: "Service stopped emitting observations — either crashed or observation pipeline blocked".to_string(),
                    recommended_action: "Check service health: verify cron jobs running, check network connectivity, review service logs".to_string(),
                    is_actionable: false,
                });
            }
            "metabolism_anomaly" => {
                patterns.push(DetectedPattern {
                    kind: "metabolism_anomaly".to_string(),
                    severity: "warning".to_string(),
                    signal: message.clone(),
                    reasoning: "Token filtering degraded — inference becoming more expensive".to_string(),
                    recommended_action: "Monitor token output in next 3 sessions; if trend continues, investigate Dog quality drift".to_string(),
                    is_actionable: false,
                });
            }
            "fleet_drift" if message.contains("offline") => {
                patterns.push(DetectedPattern {
                    kind: "fleet_drift".to_string(),
                    severity: "warning".to_string(),
                    signal: message.clone(),
                    reasoning: "Infrastructure node offline — may impact multi-node deployments".to_string(),
                    recommended_action: "Check node status via Tailscale; if persistent >10min, may need manual recovery".to_string(),
                    is_actionable: false,
                });
            }
            _ => {}
        }
    }

    patterns
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{HealthSignalPayload, SignalSeverity};

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

    #[test]
    fn typed_reconcile_signals_inference_crash_loop() {
        let signal = HealthSignal::new(
            "cynic-host-cortex",
            SignalSeverity::Critical,
            HealthSignalPayload::InferenceCrashLoop {
                service: "llama-server".to_string(),
                failure_rate_percent: 100.0,
                consecutive_failures: 6,
            },
            1_786_800_000,
        );
        let plans = reconcile_signals(&[signal], 1_786_800_005);
        assert_eq!(plans.len(), 1);
        assert!(plans[0].is_actionable);
        assert_eq!(
            plans[0].action,
            cynic_protocol::AutonomicActionKind::RestartService {
                service: "llama-server".to_string(),
            }
        );
    }

    #[test]
    fn typed_reconcile_signals_wireguard_peer_dead() {
        let signal = HealthSignal::new(
            "cynic-host-cortex",
            SignalSeverity::Critical,
            HealthSignalPayload::WireguardPeerDead {
                interface: "wg0".to_string(),
                peer_public_key_prefix: "Kar1lDbVWLydccWH".to_string(),
                age_seconds: 220,
                dead_threshold_seconds: 200,
            },
            1_786_800_000,
        );
        let plans = reconcile_signals(&[signal], 1_786_800_005);
        assert_eq!(plans.len(), 1);
        assert!(plans[0].is_actionable);
        assert_eq!(
            plans[0].action,
            cynic_protocol::AutonomicActionKind::RefreshWireguardEndpoint {
                interface: "wg0".to_string(),
                peer_pubkey_prefix: "Kar1lDbVWLydccWH".to_string(),
            }
        );
    }
}
