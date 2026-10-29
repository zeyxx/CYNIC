//! Event matching and civil date calculations (pure).

use serde_json::Value;

pub fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn event_matches(event: &Value, agent_filter: &str, target_filter: &str) -> bool {
    if event.get("schema").and_then(|v| v.as_str()) != Some("cynic.agent.claim_event.v1") {
        return false;
    }
    let agent = event.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let target = event.get("target").and_then(|v| v.as_str()).unwrap_or("");
    let agent_ok = agent_filter.is_empty() || agent_filter == "*" || agent == agent_filter;
    let target_ok = target_filter.is_empty() || target_filter == "*" || target == target_filter;
    agent_ok && target_ok
}
