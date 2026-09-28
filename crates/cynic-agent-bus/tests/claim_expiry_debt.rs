//! Claims past their own deadline, and still marked active.
//!
//! WHY: `temporal.ttl_seconds` is stored on every claim and compared against
//! nothing. 148 of 289 active claims are past their deadline — the `hermes:*`
//! agents are 13 days past — and `status` stays `active` because no code path
//! reads the clock. So the deadline is a declared property that the mechanism does
//! not apply, structurally the same defect as a boolean hard-coded `false`.
//!
//! The companion finding is that `consumed` carries no information: all 289 active
//! claims report zero consumed actions, because nothing writes that field on the
//! claim path. A field that is uniformly one value is indistinguishable from a
//! field that does not exist, so it must not be used as evidence of liveness.
//!
//! HOW: read the registry, compute the debt, assert it is no larger than the
//! recorded baseline. Shrink-only, like every ratchet in this workspace: a claim
//! released or expired by any mechanism reduces the number, and this test records
//! the improvement. The baseline may only be raised with a reason in the commit
//! that raises it.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn registry() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.agent-claims.json")
}

/// Claims that are `active`, whose deadline has passed, and were never released.
///
/// Recorded as explicit data with the debt visible. Shrink-only. At the time of
/// writing, every `hermes:*` claim is past its deadline and still active, and 46
/// of this agent's own 58 are as well — which is the point: no agent releases
/// reliably, so the fix belongs in the mechanism, not in discipline.
const EXPIRED_ACTIVE_BASELINE: usize = 165;

fn expired_active() -> Vec<(String, String, i64)> {
    let raw = std::fs::read_to_string(registry()).expect("registry readable");
    let document: serde_json::Value = serde_json::from_str(&raw).expect("registry is json");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let mut out = Vec::new();
    let empty: Vec<serde_json::Value> = Vec::new();
    let claims = document
        .get("claims")
        .and_then(|value| value.as_array())
        .unwrap_or(&empty);
    for claim in claims {
        if claim.get("status").and_then(|s| s.as_str()) != Some("active") {
            continue;
        }
        let Some(temporal) = claim.get("temporal") else {
            continue;
        };
        let started = temporal.get("started_at_unix_s").and_then(|v| v.as_i64()).unwrap_or(0);
        let ttl = temporal.get("ttl_seconds").and_then(|v| v.as_i64()).unwrap_or(0);
        let deadline = started + ttl;
        if now > deadline {
            out.push((
                claim
                    .get("agent_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                claim
                    .get("scope")
                    .and_then(|scope| scope.get("path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                now - deadline,
            ));
        }
    }
    out
}

#[test]
fn expired_active_claims_do_not_exceed_the_baseline() {
    let debt = expired_active();
    let by_agent: BTreeMap<&str, usize> = debt.iter().fold(BTreeMap::new(), |mut acc, (agent, _, _)| {
        *acc.entry(agent.as_str()).or_default() += 1;
        acc
    });
    assert!(
        debt.len() <= EXPIRED_ACTIVE_BASELINE,
        "expired-but-active claim debt grew from {EXPIRED_ACTIVE_BASELINE} to {}: {by_agent:?}\n\
         A new expiry mechanism must REDUCE this number. Raising the baseline without \
         the mechanism being applied is the failure this ratchet exists to catch.",
        debt.len()
    );
}

/// The oldest debt, named. A debt that is only a count is a debt nobody routes.
#[test]
fn the_worst_claims_are_identifiable() {
    let debt = expired_active();
    let mut worst = debt.clone();
    worst.sort_by_key(|(_, _, overdue)| std::cmp::Reverse(*overdue));
    let oldest: Vec<String> = worst
        .iter()
        .take(3)
        .map(|(agent, path, overdue)| format!("{agent} on {path}, {overdue}s overdue"))
        .collect();
    assert!(
        !oldest.is_empty(),
        "expected expired claims to exist; the registry may have changed shape"
    );
    // Reported so a failure names the holders rather than only the count.
    eprintln!("oldest expired-active claims: {}", oldest.join("; "));
}

/// `consumed` is uniformly zero, so it cannot distinguish a live claim from a
/// dead one. If that ever becomes true, this test is what noticed.
#[test]
fn the_consumed_field_is_not_usable_as_liveness_evidence() {
    let raw = std::fs::read_to_string(registry()).expect("registry readable");
    let document: serde_json::Value = serde_json::from_str(&raw).expect("registry is json");
    let empty: Vec<serde_json::Value> = Vec::new();
    let active: Vec<&serde_json::Value> = document
        .get("claims")
        .and_then(|value| value.as_array())
        .unwrap_or(&empty)
        .iter()
        .filter(|claim| claim.get("status").and_then(|s| s.as_str()) == Some("active"))
        .collect();
    let with_activity = active
        .iter()
        .filter(|claim| {
            claim
                .get("consumed")
                .and_then(|consumed| consumed.get("actions"))
                .and_then(|v| v.as_u64())
                .is_some_and(|actions| actions > 0)
        })
        .count();
    eprintln!(
        "active claims: {}, of which report any consumed action: {with_activity}",
        active.len()
    );
    assert!(
        with_activity == 0,
        "some claims now report consumed actions, so the liveness question must be \
         re-examined rather than answered by counting zeros"
    );
}
