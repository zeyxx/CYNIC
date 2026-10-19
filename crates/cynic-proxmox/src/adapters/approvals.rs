//! Human approval queue for destructive actions.
//!
//! Doctrine (`AGENCY_PRIVACY_DOCTRINE.md`): out-of-lease destructions
//! require an explicit, single-use, expiring human approval. Leased
//! actions (TTL sweep) carry consent in advance and never enter the queue.
//! The queue is data; any dashboard (CLI today, TUI later) renders it.
//!
//! Lifecycle: propose → (list) → approve → execute-once → consumed.
//! Expired or unknown intents refuse. Execution itself lives with the
//! action owners (CLI arms); this module owns the gate.

use serde::{Deserialize, Serialize};

/// Closed set of approvable destructive actions. Adding a variant is a
/// design decision, never a runtime extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructiveAction {
    /// Delete a Freebox NAT/port-forwarding rule by id.
    FreeboxFwRedirDelete { rule_id: u32 },
    /// Destroy an LXC guest outside any lease (unleased, retired template).
    ProxmoxLxcDestroy { vmid: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStatus {
    Pending,
    Approved,
    Consumed,
    Expired,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestructiveIntent {
    pub id: String,
    pub action: DestructiveAction,
    pub reason: String,
    pub proposed_by: String,
    pub proposed_at: u64,
    pub expires_at: u64,
    pub status: IntentStatus,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Propose an intent. TTL bounded (max 72 h); the id is derived, not chosen.
pub fn propose_intent(
    action: DestructiveAction,
    reason: &str,
    proposed_by: &str,
    ttl_hours: u64,
) -> Result<DestructiveIntent, String> {
    if reason.trim().is_empty() {
        return Err("approval_reason_required".to_string());
    }
    if ttl_hours == 0 || ttl_hours > 72 {
        return Err("approval_ttl_out_of_bounds:max_72h".to_string());
    }
    let proposed_at = now_unix();
    let id = format!(
        "intent-{}-{:x}",
        proposed_at,
        {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            format!("{action:?}{reason}{proposed_by}").hash(&mut hasher);
            hasher.finish()
        }
    );
    Ok(DestructiveIntent {
        id,
        action,
        reason: reason.to_string(),
        proposed_by: proposed_by.to_string(),
        proposed_at,
        expires_at: proposed_at + ttl_hours * 3600,
        status: IntentStatus::Pending,
    })
}

/// Read the queue file. Missing file means empty queue (not an error).
pub fn read_queue(path: &std::path::Path) -> Result<Vec<DestructiveIntent>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("approvals_unreadable:{}:{error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("approvals_corrupt:{}:{error}", path.display()))
}

fn write_queue(path: &std::path::Path, intents: &[DestructiveIntent]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("approvals_mkdir:{}:{error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(intents)
        .map_err(|error| format!("approvals_encode:{error}"))?;
    std::fs::write(path, text)
        .map_err(|error| format!("approvals_unwritable:{}:{error}", path.display()))
}

/// Append a proposed intent to the queue.
pub fn enqueue(path: &std::path::Path, intent: &DestructiveIntent) -> Result<(), String> {
    let mut intents = read_queue(path)?;
    if intents.iter().any(|existing| existing.id == intent.id) {
        return Err("approval_duplicate_intent".to_string());
    }
    intents.push(intent.clone());
    write_queue(path, &intents)
}

/// Human approval: pending → approved. Anything else refuses.
pub fn approve(path: &std::path::Path, id: &str, now: u64) -> Result<DestructiveIntent, String> {
    let mut intents = read_queue(path)?;
    let intent = intents
        .iter_mut()
        .find(|intent| intent.id == id)
        .ok_or_else(|| "approval_unknown_intent".to_string())?;
    if intent.status != IntentStatus::Pending {
        return Err(format!("approval_not_pending:{:?}", intent.status));
    }
    if now >= intent.expires_at {
        intent.status = IntentStatus::Expired;
        write_queue(path, &intents)?;
        return Err("approval_expired".to_string());
    }
    intent.status = IntentStatus::Approved;
    let approved = intent.clone();
    write_queue(path, &intents)?;
    Ok(approved)
}

/// Consume an approved, unexpired intent for one execution. Returns the
/// action on success and marks it consumed so it can never run twice.
pub fn consume(path: &std::path::Path, id: &str, now: u64) -> Result<DestructiveAction, String> {
    let mut intents = read_queue(path)?;
    let intent = intents
        .iter_mut()
        .find(|intent| intent.id == id)
        .ok_or_else(|| "approval_unknown_intent".to_string())?;
    if intent.status != IntentStatus::Approved {
        return Err(format!("approval_not_approved:{:?}", intent.status));
    }
    if now >= intent.expires_at {
        intent.status = IntentStatus::Expired;
        write_queue(path, &intents)?;
        return Err("approval_expired".to_string());
    }
    intent.status = IntentStatus::Consumed;
    let action = intent.action.clone();
    write_queue(path, &intents)?;
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("cynic-approvals-test-{name}.json"))
    }

    #[test]
    fn full_lifecycle_propose_approve_consume_once() {
        let path = test_path("lifecycle");
        let _ = std::fs::remove_file(&path);
        let intent = propose_intent(
            DestructiveAction::FreeboxFwRedirDelete { rule_id: 2 },
            "close public 8006, no external consumer in 2 days of logs",
            "hermes",
            24,
        )
        .expect("propose");
        enqueue(&path, &intent).expect("enqueue");

        let approved = approve(&path, &intent.id, intent.proposed_at + 60).expect("approve");
        assert_eq!(approved.status, IntentStatus::Approved);

        let action = consume(&path, &intent.id, intent.proposed_at + 120).expect("consume");
        assert_eq!(action, DestructiveAction::FreeboxFwRedirDelete { rule_id: 2 });

        let again = consume(&path, &intent.id, intent.proposed_at + 180);
        assert!(again.is_err(), "double execution must refuse");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn expired_approvals_never_execute() {
        let path = test_path("expiry");
        let _ = std::fs::remove_file(&path);
        let intent = propose_intent(
            DestructiveAction::ProxmoxLxcDestroy { vmid: 100 },
            "unleased guest, owner unknown",
            "hermes",
            1,
        )
        .expect("propose");
        enqueue(&path, &intent).expect("enqueue");

        let late = approve(&path, &intent.id, intent.expires_at + 1);
        assert!(late.is_err(), "late approval must refuse");
        let stored = read_queue(&path).expect("read");
        assert_eq!(stored[0].status, IntentStatus::Expired);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_reason_and_wild_ttl_refused() {
        assert!(propose_intent(
            DestructiveAction::FreeboxFwRedirDelete { rule_id: 2 },
            "",
            "hermes",
            24,
        )
        .is_err());
        assert!(propose_intent(
            DestructiveAction::FreeboxFwRedirDelete { rule_id: 2 },
            "x",
            "hermes",
            0,
        )
        .is_err());
        assert!(propose_intent(
            DestructiveAction::FreeboxFwRedirDelete { rule_id: 2 },
            "x",
            "hermes",
            73,
        )
        .is_err());
    }
}
