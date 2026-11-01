//! Per-Dog Prometheus metrics exposition.
//!
//! Ported from `cynic-host` legacy metrics module. Pure text formatting,
//! no external crate dependencies.

use std::fmt::Write;

/// Append per-dog metrics from usage tracker data.
///
/// Called by the /metrics handler with data from DogUsageTracker::merged_dogs().
pub fn append_dog_metrics(
    out: &mut String,
    dogs: &[(String, u64, u64, u64, u64)], // (id, requests, failures, total_latency_ms, total_tokens)
    circuit_states: &[(String, String, u32)], // (id, state, consecutive_failures)
) {
    // Per-dog request count
    let _ = writeln!(
        out,
        "# HELP cynic_dog_requests_total Total requests per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_requests_total counter");
    for (id, requests, _, _, _) in dogs {
        let _ = writeln!(out, "cynic_dog_requests_total{{dog=\"{id}\"}} {requests}");
    }

    // Per-dog failure count
    let _ = writeln!(out, "# HELP cynic_dog_failures Per-dog evaluation failures");
    let _ = writeln!(out, "# TYPE cynic_dog_failures counter");
    for (id, _, failures, _, _) in dogs {
        let _ = writeln!(out, "cynic_dog_failures{{dog=\"{id}\"}} {failures}");
    }

    // Per-dog token consumption
    let _ = writeln!(
        out,
        "# HELP cynic_dog_tokens_total Total tokens consumed per Dog"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_tokens_total counter");
    for (id, _, _, _, tokens) in dogs {
        let _ = writeln!(out, "cynic_dog_tokens_total{{dog=\"{id}\"}} {tokens}");
    }

    // Circuit breaker state: 0=closed, 1=open, 2=half-open
    let _ = writeln!(
        out,
        "# HELP cynic_dog_circuit_breaker Circuit breaker state (0=closed, 1=open, 2=half-open)"
    );
    let _ = writeln!(out, "# TYPE cynic_dog_circuit_breaker gauge");
    for (id, state, _) in circuit_states {
        let val = match state.as_str() {
            "closed" => 0,
            "open" | "critical" => 1,
            "half-open" => 2,
            _ => 0,
        };
        let _ = writeln!(out, "cynic_dog_circuit_breaker{{dog=\"{id}\"}} {val}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_dog_metrics_formats_correctly() {
        let mut out = String::new();
        let dogs = vec![
            ("gemini".to_string(), 10u64, 2u64, 5000u64, 15000u64),
            ("sovereign".to_string(), 5u64, 0u64, 10000u64, 8000u64),
        ];
        let circuits = vec![
            ("gemini".to_string(), "closed".to_string(), 0u32),
            ("sovereign".to_string(), "open".to_string(), 3u32),
        ];
        append_dog_metrics(&mut out, &dogs, &circuits);
        assert!(out.contains("cynic_dog_requests_total{dog=\"gemini\"} 10"));
        assert!(out.contains("cynic_dog_circuit_breaker{dog=\"sovereign\"} 1"));
    }
}
